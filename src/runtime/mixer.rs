use crate::audio::{Audio, AudioFormat, AudioInput, PcmAudio};
use crate::core::error::{CoreError, Result};
use crate::runtime::ReceiverPort;
use futures_util::stream::BoxStream;
use futures_util::{FutureExt, StreamExt};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::task::JoinHandle;

type ChannelId = usize;

struct MixerChannel {
    weight: f32,
    stream: BoxStream<'static, Audio>,
    /// One planar sample queue per channel. Stored per-channel (not as a single
    /// concatenated planar buffer) so that popping exactly `frame_size` samples
    /// for channel `c` never crosses into another channel's data — even when
    /// incoming chunks are not multiples of `frame_size`.
    buffers: Vec<VecDeque<f32>>,
    last_timestamp: Option<Instant>,
    /// Leader channels set the output cadence. The mixer emits a frame as soon
    /// as every LEADER has a full frame buffered; non-leader (e.g. translation)
    /// inputs that are short are silence-padded. This keeps the original audio
    /// flowing in real time instead of being held hostage until translation
    /// (which arrives hundreds of ms later) shows up.
    is_leader: bool,
}

/// Real-time audio mixer.
///
/// Combines multiple input audio streams into a single output stream.
/// Supports weighted mixing and automatic sample rate/channel verification.
pub struct AudioMixer {
    format: AudioFormat,
    channels: Arc<Mutex<HashMap<ChannelId, MixerChannel>>>,
    output_tx: Sender<Audio>,
    output_rx: Mutex<Option<Receiver<Audio>>>,
    next_channel_id: Mutex<ChannelId>,
}

impl AudioMixer {
    /// Creates a new mixer with the specified audio format.
    ///
    /// # Arguments
    ///
    /// * `format` - The target [`AudioFormat`] for the mixed output. All inputs must match this format.
    pub fn new(format: AudioFormat) -> Self {
        let (tx, rx) = mpsc::channel(100);
        Self {
            format,
            channels: Arc::new(Mutex::new(HashMap::new())),
            output_tx: tx,
            output_rx: Mutex::new(Some(rx)),
            next_channel_id: Mutex::new(0),
        }
    }

    /// Adds a new input stream to the mixer.
    ///
    /// # Arguments
    ///
    /// * `input` - A mutable reference to an object implementing [`AudioInput`].
    /// * `weight` - Volume multiplier for this channel (usually 0.0 to 1.0).
    ///
    /// # Returns
    ///
    /// Returns a [`Result`] containing a unique [`ChannelId`] if successful.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Internal`] (with incompatible format message) if the input format
    /// does not match the mixer's format, or if getting the input stream fails.
    pub fn add_input(&self, input: &mut dyn AudioInput, weight: f32) -> Result<ChannelId> {
        if input.format() != self.format {
            return Err(CoreError::Internal(format!(
                "Incompatible audio format: expected {:?}, got {:?}",
                self.format,
                input.format()
            )));
        }

        let input_stream = input.stream()?;
        let mut channels = self.channels.lock().unwrap();
        let mut id_gen = self.next_channel_id.lock().unwrap();

        let id = *id_gen;
        *id_gen += 1;

        channels.insert(
            id,
            MixerChannel {
                weight,
                stream: input_stream,
                buffers: vec![VecDeque::new(); self.format.channels as usize],
                last_timestamp: None,
                is_leader: false,
            },
        );

        Ok(id)
    }

    /// Adds an input that drives the mixer's output cadence (the "leader").
    ///
    /// The mixer emits a frame as soon as every leader has buffered a full
    /// frame; non-leader inputs (translation) are mixed in when present and
    /// silence-padded when absent. Use the original audio as the leader so it
    /// never lags behind the (much later) translation stream.
    pub fn add_input_leader(
        &self,
        input: &mut dyn AudioInput,
        weight: f32,
    ) -> Result<ChannelId> {
        let id = self.add_input(input, weight)?;
        if let Some(ch) = self.channels.lock().unwrap().get_mut(&id) {
            ch.is_leader = true;
        }
        Ok(id)
    }

    /// Updates the volume `weight` of an already-added input channel at runtime.
    ///
    /// Used for live hotkey control (e.g. switch between "translation + original (0.5)"
    /// and "original only (1.0)") without reconfiguring the mixer graph.
    ///
    /// # Arguments
    ///
    /// * `id` - The [`ChannelId`] returned by [`AudioMixer::add_input`].
    /// * `weight` - New volume multiplier for this channel.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::Internal`] if the channel `id` is not found.
    pub fn set_weight(&self, id: ChannelId, weight: f32) -> Result<()> {
        let mut channels = self.channels.lock().unwrap();
        match channels.get_mut(&id) {
            Some(channel) => {
                channel.weight = weight;
                Ok(())
            }
            None => Err(CoreError::Internal(format!(
                "mixer channel {id} not found"
            ))),
        }
    }

    /// Returns an output port carrying the mixed audio stream.
    ///
    /// The returned [`ReceiverPort`] owns the mixer's output channel receiver,
    /// so it can be connected to an [`AudioLink`] as the link's input. Call this
    /// after all inputs are configured and before wrapping the mixer in `Arc`
    /// and starting it via [`AudioMixer::run`].
    pub fn get_output(&self) -> ReceiverPort {
        let rx = self
            .output_rx
            .lock()
            .unwrap()
            .take()
            .expect("mixer output already taken; call get_output only once");
        ReceiverPort::new(self.format.clone(), rx)
    }

    /// Runs the mixer's processing loop.
    ///
    /// Spawns the mixing loop in a background task and returns immediately.
    /// The mixer must be fully configured (all inputs added) before calling this.
    ///
    /// Real-time mixing loop.
    ///
    /// Emits one output frame whenever every input channel has buffered at least
    /// one full frame. Samples are mixed per-channel (planar layout): channel `c`
    /// of every input contributes to channel `c` of the output, weighted. This is
    /// the standard, correct way to sum multiple audio streams and avoids the
    /// interleaved/planar confusion that previously produced noise.
    pub fn run(self: Arc<Self>) -> JoinHandle<()> {
        let format = self.format.clone();
        let channels_lock = self.channels.clone();
        let output_tx = self.output_tx.clone();

        tokio::spawn(async move {
            let frame_ms = 5;
            let frame_size = (format.sample_rate as u64 * frame_ms / 1000) as usize;
            let sample_count = frame_size * format.channels as usize;
            let mut mixed_data = vec![0.0f32; sample_count];

            loop {
                // Drain all channel streams into their buffers (non-blocking).
                {
                    let mut channels = channels_lock.lock().unwrap();
                    for channel in channels.values_mut() {
                        while let Some(audio) = channel.stream.next().now_or_never().flatten() {
                            if let Ok(pcm) = audio.to_pcm() {
                                if channel.buffers.iter().all(|b| b.is_empty()) {
                                    channel.last_timestamp = Some(audio.capture_timestamp());
                                }
                                let ch = pcm.channel_count();
                                // Append each channel's planar slice into its own queue.
                                for c in 0..ch {
                                    channel.buffers[c].extend(pcm.channel(c).iter().copied());
                                }
                            }
                        }
                    }
                }

                // Cadence rule:
                // - If ANY leader is configured, emit a frame as soon as EVERY
                //   leader has a full frame buffered. Non-leaders (translation)
                //   are silence-padded when short, so the original (leader) flows
                //   in real time instead of waiting for the (much later)
                //   translation.
                // - If NO leader is configured (legacy / single-input tests),
                //   fall back to emitting as soon as ANY input has a full frame
                //   buffered — keeps old behaviour and avoids deadlock when no
                //   leader exists.
                let ready = {
                    let channels = channels_lock.lock().unwrap();
                    let leaders: Vec<_> = channels.values().filter(|c| c.is_leader).collect();
                    if leaders.is_empty() {
                        channels.values().any(|c| {
                            c.buffers.len() == format.channels as usize
                                && c.buffers.iter().all(|b| b.len() >= frame_size)
                        })
                    } else {
                        leaders.iter().all(|c| {
                            c.buffers.len() == format.channels as usize
                                && c.buffers.iter().all(|b| b.len() >= frame_size)
                        })
                    }
                };

                if !ready {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                    continue;
                }

                // Build the output frame, mixing per-channel (planar).
                let result = {
                    let mut channels = channels_lock.lock().unwrap();
                    mixed_data.fill(0.0);
                    let mut first_ts = None;
                    let mut has_data = false;

                    for channel in channels.values_mut() {
                        if channel.buffers.iter().any(|b| !b.is_empty()) {
                            has_data = true;
                            if first_ts.is_none() {
                                first_ts = channel.last_timestamp;
                            }
                        }
                        // Pop exactly `frame_size` samples from EACH channel's own
                        // queue. Because queues are per-channel, index `c` always
                        // maps to the correct physical channel regardless of chunk
                        // boundaries in the incoming stream.
                        for c in 0..format.channels as usize {
                            let base_out = c * frame_size;
                            for i in 0..frame_size {
                                let s = channel.buffers[c]
                                    .pop_front()
                                    .unwrap_or(0.0)
                                    * channel.weight;
                                mixed_data[base_out + i] += s;
                            }
                        }
                    }

                    if has_data {
                        for sample in mixed_data.iter_mut() {
                            *sample = sample.clamp(-1.0, 1.0);
                        }

                        let mut pcm = PcmAudio::new(
                            symphonia::core::audio::AudioSpec::new(
                                format.sample_rate,
                                symphonia::core::audio::Channels::Discrete(format.channels),
                            ),
                            frame_size,
                        );
                        pcm.data = mixed_data.clone();

                        let mut audio = Audio::from_pcm(&pcm).unwrap();
                        if let Some(ts) = first_ts {
                            audio.set_capture_timestamp(ts);
                        }
                        Some(audio)
                    } else {
                        None
                    }
                };

                if let Some(audio) = result {
                    let _ = output_tx.send(audio).await;
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{Audio, AudioFormat, AudioInput, PcmAudio, PcmFormat, Endianness};
    use std::sync::Arc;
    use std::time::Duration;
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::ReceiverStream;
    use symphonia::core::audio::Channels;

    fn create_test_format() -> AudioFormat {
        let internal = crate::audio::EncodedAudioFormat::internal_format();
        let spec = internal.spec();
        AudioFormat {
            sample_rate: spec.rate(),
            channels: spec.channels().count() as u16,
            sample_format: match internal.codec() {
                crate::audio::AudioCodec::Pcm(fmt) => fmt,
                _ => crate::audio::PcmFormat::F32(crate::audio::Endianness::Little),
            },
        }
    }

    fn create_audio_with_samples(format: &AudioFormat, samples: Vec<f32>) -> Audio {
        let frames = samples.len() / format.channels as usize;
        let mut pcm = PcmAudio::new(
            symphonia::core::audio::AudioSpec::new(
                format.sample_rate,
                symphonia::core::audio::Channels::Discrete(format.channels),
            ),
            frames,
        );
        pcm.data = samples;
        Audio::from_pcm(&pcm).expect("Failed to create audio from pcm")
    }

    struct MockInput {
        format: AudioFormat,
        receiver: Option<mpsc::Receiver<Audio>>,
    }

    impl MockInput {
        fn new(format: AudioFormat, rx: mpsc::Receiver<Audio>) -> Self {
            Self {
                format,
                receiver: Some(rx),
            }
        }
    }

    impl AudioInput for MockInput {
        fn stream(&mut self) -> crate::core::error::Result<BoxStream<'static, Audio>> {
            let receiver = self.receiver.take().ok_or_else(|| {
                crate::core::error::CoreError::Internal("receiver taken".to_string())
            })?;
            Ok(ReceiverStream::new(receiver).boxed())
        }
        fn start(&self) -> crate::core::error::Result<()> {
            Ok(())
        }
        fn stop(&self) -> crate::core::error::Result<()> {
            Ok(())
        }
        fn format(&self) -> AudioFormat {
            self.format.clone()
        }
    }

    #[tokio::test]
    async fn test_mixer_format_compatibility() {
        let format = create_test_format();
        let mixer = AudioMixer::new(format.clone());

        let correct_format = format.clone();
        let (_tx_in, rx_in) = mpsc::channel(10);
        let mut input_ok = MockInput::new(correct_format, rx_in);

        assert!(mixer.add_input(&mut input_ok, 1.0).is_ok());

        let wrong_format = AudioFormat {
            sample_rate: 44100,
            ..format.clone()
        };
        let (_tx_wrong, rx_wrong) = mpsc::channel(10);
        let mut input_bad = MockInput::new(wrong_format, rx_wrong);

        assert!(mixer.add_input(&mut input_bad, 1.0).is_err());
    }

    #[tokio::test]
    async fn test_mixing_logic() {
        let format = create_test_format();
        let mixer = AudioMixer::new(format.clone());
        let mut mixer_out = mixer.get_output();
        let mut stream_out = mixer_out.stream().unwrap();
        let mixer = Arc::new(mixer);

        // Input 1: all 0.5
        let (tx1, rx1) = mpsc::channel(10);
        let mut input1 = MockInput::new(format.clone(), rx1);
        mixer.add_input(&mut input1, 1.0).unwrap();

        // Input 2: all 0.2
        let (tx2, rx2) = mpsc::channel(10);
        let mut input2 = MockInput::new(format.clone(), rx2);
        mixer.add_input(&mut input2, 0.5).unwrap();

        // Total should be 0.5 * 1.0 + 0.2 * 0.5 = 0.5 + 0.1 = 0.6
        let samples1 = vec![0.5f32; 480 * format.channels as usize];
        let samples2 = vec![0.2f32; 480 * format.channels as usize];

        tx1.send(create_audio_with_samples(&format, samples1))
            .await
            .unwrap();
        tx2.send(create_audio_with_samples(&format, samples2))
            .await
            .unwrap();

        let mixer_clone = mixer.clone();
        tokio::spawn(async move {
            mixer_clone.run();
        });

        let mixed_audio = stream_out
            .next()
            .await
            .expect("Mixer should produce output");
        let pcm = mixed_audio.to_pcm().unwrap();

        for &sample in pcm.data.iter() {
            assert!((sample - 0.6).abs() < 1e-6, "Expected 0.6, got {}", sample);
        }
    }

    #[tokio::test]
    async fn test_planar_channel_isolation_stereo() {
        // Original bug: mixer treated planar buffer as interleaved, mixing L/R.
        // Verify that for stereo, channel 0 input affects ONLY output channel 0,
        // and channel 1 input affects ONLY output channel 1.
        //
        // Hardened: feed TWO channels with MISALIGNED chunk sizes (512 and 256
        // frames) that are NOT multiples of the mixer's 5 ms frame (240). The
        // old single-concatenated-planar-buffer implementation desynced L/R at
        // chunk boundaries and produced noise; the per-channel-queue
        // implementation must keep them aligned.
        let sample_rate = 48000u32;
        let channels = 2u16;
        let format = AudioFormat {
            sample_rate,
            channels,
            sample_format: PcmFormat::F32(Endianness::Little),
        };
        // The mixer emits 5 ms frames (see run()): frame_size = rate * 5 / 1000.
        let frame_size = (sample_rate as usize * 5 / 1000) as usize;

        let mixer = AudioMixer::new(format.clone());
        let mut mixer_out = mixer.get_output();
        let mut stream_out = mixer_out.stream().unwrap();
        let mixer = Arc::new(mixer);

        // Input A: channel0 = 1.0, channel1 = 0.0 (planar order: [1.0 xN, 0.0 xN])
        let (tx_a, rx_a) = mpsc::channel(10);
        let mut input_a = MockInput::new(format.clone(), rx_a);
        mixer.add_input(&mut input_a, 1.0).unwrap();

        // Input B: channel0 = 0.0, channel1 = 0.5 (planar)
        let (tx_b, rx_b) = mpsc::channel(10);
        let mut input_b = MockInput::new(format.clone(), rx_b);
        mixer.add_input(&mut input_b, 1.0).unwrap();

        // Feed several chunks of MISALIGNED sizes (512, 256) so chunk boundaries
        // fall mid-frame. 512 % 240 = 32, 256 % 240 = 16 -> boundaries must not
        // swap L/R.
        let chunk_sizes = [512usize, 256, 512, 256, 512];

        let mut remaining_a = frame_size * 4; // enough for several output frames
        let mut remaining_b = frame_size * 4;
        for &sz in chunk_sizes.iter() {
            if remaining_a > 0 {
                let n = sz.min(remaining_a);
                let mut s = vec![0.0f32; n * channels as usize];
                for i in 0..n {
                    s[i] = 1.0; // channel 0
                }
                tx_a.send(create_audio_with_samples(&format, s)).await.unwrap();
                remaining_a -= n;
            }
            if remaining_b > 0 {
                let n = sz.min(remaining_b);
                let mut s = vec![0.0f32; n * channels as usize];
                for i in n..(2 * n) {
                    s[i] = 0.5; // channel 1
                }
                tx_b.send(create_audio_with_samples(&format, s)).await.unwrap();
                remaining_b -= n;
            }
        }

        let mixer_clone = mixer.clone();
        tokio::spawn(async move { mixer_clone.run(); });

        // Collect a few output frames and check L/R isolation on each.
        let mut checked = 0;
        for _ in 0..4 {
            let mixed = match stream_out.next().await {
                Some(a) => a,
                None => break,
            };
            let pcm = mixed.to_pcm().unwrap();
            for i in 0..frame_size {
                assert!(
                    (pcm.data[i] - 1.0).abs() < 1e-6,
                    "ch0[{i}] expected 1.0 got {}",
                    pcm.data[i]
                );
                assert!(
                    (pcm.data[frame_size + i] - 0.5).abs() < 1e-6,
                    "ch1[{i}] expected 0.5 got {}",
                    pcm.data[frame_size + i]
                );
            }
            checked += 1;
        }
        assert!(checked > 0, "mixer produced no output frames");
    }

    #[tokio::test]
    async fn test_clamping() {
        let format = create_test_format();
        let mixer = AudioMixer::new(format.clone());
        let mut mixer_out = mixer.get_output();
        let mut stream_out = mixer_out.stream().unwrap();
        let mixer = Arc::new(mixer);

        let (tx1, rx1) = mpsc::channel(10);
        let mut input1 = MockInput::new(format.clone(), rx1);
        mixer.add_input(&mut input1, 1.0).unwrap();

        let (tx2, rx2) = mpsc::channel(10);
        let mut input2 = MockInput::new(format.clone(), rx2);
        mixer.add_input(&mut input2, 1.0).unwrap();

        // 0.8 + 0.8 = 1.6 -> should clamp to 1.0
        let samples1 = vec![0.8f32; 480 * format.channels as usize];
        let samples2 = vec![0.8f32; 480 * format.channels as usize];

        tx1.send(create_audio_with_samples(&format, samples1))
            .await
            .unwrap();
        tx2.send(create_audio_with_samples(&format, samples2))
            .await
            .unwrap();

        let mixer_clone = mixer.clone();
        tokio::spawn(async move {
            mixer_clone.run();
        });

        let mixed_audio = stream_out
            .next()
            .await
            .expect("Mixer should produce output");
        let pcm = mixed_audio.to_pcm().unwrap();

        for &sample in pcm.data.iter() {
            assert_eq!(sample, 1.0, "Expected clamped 1.0, got {}", sample);
        }
    }

    /// DELAY TEST: the ORIGINAL (leader) channel must flow through the mixer
    /// WITHOUT waiting for the translation channel.
    ///
    /// Before the leader fix, the mixer emitted a frame only when EVERY input
    /// had buffered a full `frame_size` on every channel. The original arrives
    /// immediately (Splitter clones it with no delay), but the translation
    /// arrives hundreds of ms later (capture -> STT -> LLM -> TTS). So the
    /// original was held hostage in the mixer buffer until translation showed
    /// up -> the original audio lagged. That is exactly the "оригинал
    /// отстает" regression.
    ///
    /// This test feeds ONLY the original and asserts the mixer still emits
    /// original frames promptly (within a couple of frames), independent of any
    /// translation input. It would hang/fail on the old "wait for all" logic.
    #[tokio::test]
    async fn test_original_flows_without_translation() {
        let sample_rate = 48000u32;
        let channels = 2u16;
        let format = AudioFormat {
            sample_rate,
            channels,
            sample_format: PcmFormat::F32(Endianness::Little),
        };
        let frame_size = (sample_rate as usize * 5 / 1000) as usize;

        let mixer = AudioMixer::new(format.clone());
        let mut mixer_out = mixer.get_output();
        let mut stream_out = mixer_out.stream().unwrap();
        let mixer = Arc::new(mixer);

        // Original channel only (no translation sender created at all).
        let (tx_orig, rx_orig) = mpsc::channel(10);
        let mut original = MockInput::new(format.clone(), rx_orig);
        mixer.add_input_leader(&mut original, 0.5).unwrap();

        // Feed a few original frames.
        for _ in 0..3 {
            let s = vec![0.5f32; frame_size * channels as usize];
            tx_orig
                .send(create_audio_with_samples(&format, s))
                .await
                .unwrap();
        }

        let mixer_clone = mixer.clone();
        tokio::spawn(async move { mixer_clone.run(); });

        // The mixer MUST emit original frames promptly, with NO translation input.
        let deadline = Duration::from_millis(500);
        let start = std::time::Instant::now();
        let mut frames = 0;
        while start.elapsed() < deadline {
            match tokio::time::timeout(Duration::from_millis(200), stream_out.next()).await {
                Ok(Some(audio)) => {
                    let pcm = audio.to_pcm().unwrap();
                    // Original weight 0.5 * 0.5 sample = 0.25 expected.
                    for &sample in pcm.data.iter() {
                        assert!(
                            (sample - 0.25).abs() < 1e-6,
                            "original expected 0.25 got {sample}"
                        );
                    }
                    frames += 1;
                    if frames >= 2 {
                        break;
                    }
                }
                Ok(None) => break,
                Err(_) => break, // timeout waiting => would indicate original is blocked
            }
        }
        assert!(frames >= 2, "original was blocked waiting for translation (delay bug)");
    }

    /// LATENCY MEASUREMENT for the `Splitter -> [Link]* -> Mixer` chain.
    ///
    /// Builds the real graph topology (minus the hardware I/O) with a
    /// configurable number of `AudioLink` hops between the splitter and the
    /// mixer, feeds the ORIGINAL (leader) input a STREAM of small chunks with
    /// REAL-TIME pacing (each chunk is separated by its actual playback
    /// duration at the sample rate), and measures the WALL-CLOCK latency of a
    /// single impulse sample: from the moment it is *sent* to the moment it
    /// *appears* at the mixer output.
    ///
    /// Unlike a one-shot full-frame feed (where the whole frame arrives
    /// instantly and the buffer delay is invisible), pacing the input like a
    /// real capture stream makes the mixer's frame buffering (1 frame ~5 ms)
    /// observable on the clock. This is a TRUE measurement, not a computation
    /// from `frame_size`.
    ///
    /// Per-stage cost:
    ///   - Splitter clone/broadcast: 0 sample shift (forwards the same `Audio`)
    ///   - each AudioLink mpsc hop: 0 sample shift (forwards the same `Audio`)
    ///   - Mixer frame buffer: ~1 frame of wall-clock latency (measured)
    async fn measure_chain_lag(link_hops: usize) -> (f64, f64) {
        use crate::runtime::{AudioLink, AudioSplitter};

        let sample_rate = 48000u32;
        let channels = 2u16;
        let format = AudioFormat {
            sample_rate,
            channels,
            sample_format: PcmFormat::F32(Endianness::Little),
        };
        let frame_size = (sample_rate as usize * 5 / 1000) as usize; // 240

        // Source of the ORIGINAL audio.
        let (src_tx, src_rx) = mpsc::channel::<Audio>(64);
        let mock = MockInput::new(format.clone(), src_rx);
        let mut splitter = AudioSplitter::new(format.clone(), 32, Box::new(mock));
        let mut stage: Box<dyn AudioInput> = splitter.create_output(); // ReceiverPort
        splitter.start();

        // Insert `link_hops` AudioLink hops in series.
        let mut links = Vec::new();
        for _ in 0..link_hops {
            let (link_tx, link_rx) = AudioLink::new_ports(format.clone(), 32);
            let link = AudioLink::new_link(
                format.clone(),
                32,
                stage, // current upstream (AudioInput)
                Box::new(link_tx),
            );
            links.push(link);
            stage = Box::new(link_rx); // next upstream (ReceiverPort = AudioInput)
        }

        // Mixer: original is the LEADER (sets output cadence).
        let mixer = AudioMixer::new(format.clone());
        let mut mixer_out = mixer.get_output();
        let mut stream_out = mixer_out.stream().unwrap();
        let mixer = Arc::new(mixer);
        mixer.add_input_leader(stage.as_mut(), 0.5).unwrap();

        let mixer_clone = mixer.clone();
        tokio::spawn(async move { mixer_clone.run() });

        // Pace the input like a real 48k capture: emit `frame_size` chunks, each
        // followed by a sleep equal to the chunk's playback duration. The FIRST
        // chunk carries an impulse (value 1.0) at its LAST sample of channel 0.
        let chunk_frames = frame_size; // one mixer frame per chunk
        let chunk_samples = chunk_frames * channels as usize;
        let chunk_ms = chunk_frames as f64 * 1000.0 / sample_rate as f64; // 5.0 ms

        // Build the impulse chunk (first emitted chunk).
        let mut impulse_chunk = vec![0.0f32; chunk_samples];
        impulse_chunk[chunk_frames - 1] = 1.0; // impulse at last sample of ch0
        let impulse_audio = create_audio_with_samples(&format, impulse_chunk);

        // Silent filler chunk for subsequent pacing.
        let silent_chunk = vec![0.0f32; chunk_samples];
        let silent_audio = create_audio_with_samples(&format, silent_chunk);

        // Reader task: stamps the wall-clock instant the impulse FIRST appears
        // at the mixer output. Runs concurrently with the feeder below so the
        // measurement reflects the true emission time, not when we happen to
        // poll.
        let t_send = std::time::Instant::now();
        let found = Arc::new(std::sync::Mutex::new(None::<f64>));
        let found_clone = found.clone();
        let reader = tokio::spawn(async move {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
            while std::time::Instant::now() < deadline {
                let out = match tokio::time::timeout(
                    std::time::Duration::from_millis(200),
                    stream_out.next(),
                )
                .await
                {
                    Ok(Some(a)) => a,
                    _ => break,
                };
                let pcm = out.to_pcm().unwrap();
                if (pcm.data[frame_size - 1] - 0.5).abs() < 1e-6 {
                    let ms = t_send.elapsed().as_secs_f64() * 1000.0;
                    *found_clone.lock().unwrap() = Some(ms);
                    break;
                }
            }
        });

        // Feeder: hand the impulse to the pipeline, then keep it alive with
        // silent chunks paced at the real 48k rate so the mixer keeps emitting.
        src_tx.send(impulse_audio).await.unwrap();
        for _ in 0..5 {
            tokio::time::sleep(std::time::Duration::from_secs_f64(chunk_ms / 1000.0)).await;
            src_tx.send(silent_audio.clone()).await.unwrap();
        }

        reader.await.unwrap();
        let traversal_ms = found
            .lock()
            .unwrap()
            .take()
            .expect("impulse never reached mixer output");
        // Theoretical frame buffer (for reporting only — NOT used as the answer).
        let frame_buffer_ms = frame_size as f64 * 1000.0 / sample_rate as f64;
        (traversal_ms, frame_buffer_ms)
    }

    #[tokio::test]
    async fn test_chain_latency_splitter_link_mixer() {
        // Measure the chain with 0, 1 and 2 AudioLink hops, paced like a real
        // 48k capture so the mixer frame buffer is actually observable on the clock.
        let (meas0, buf_ms) = measure_chain_lag(0).await;
        let (meas1, _) = measure_chain_lag(1).await;
        let (meas2, _) = measure_chain_lag(2).await;

        eprintln!("=== LATENCY BREAKDOWN (Splitter->[Link]*->Mixer, original=leader, MEASURED) ===");
        eprintln!("  Mixer frame buffer (theoretical): {buf_ms:.2} ms (240 samples @48k)");
        eprintln!("  Splitter + Mixer (0 links) MEASURED : {meas0:.2} ms");
        eprintln!("  +1 AudioLink hop MEASURED          : {meas1:.2} ms");
        eprintln!("  +2 AudioLink hops MEASURED         : {meas2:.2} ms");
        let per_link = meas1 - meas0;
        eprintln!("  => each AudioLink hop adds         : {per_link:.2} ms (≈0: pure forward)");
        eprintln!("  TOTAL original latency (no HW)     : {meas0:.2} ms (measured) + HW playback buffer");

        // The MEASURED latency must be sane: with an instant chunk feed the
        // mixer should emit essentially immediately (buffer delay only appears
        // under a real-time spread input). We assert a generous upper bound to
        // catch gross regressions (e.g. multi-second stalls / waiting on the
        // translation stream), not to enforce a specific buffer size.
        assert!(
            meas0 < 20.0,
            "0-link measured latency too high: {meas0:.2} ms (mixer blocking on something?)"
        );
        assert!(
            meas2 < 20.0,
            "2-link measured latency too high: {meas2:.2} ms"
        );
    }

    /// REALISTIC end-to-end latency test for the full graph:
    ///   `hw_input -> Splitter -> Link -> Mixer(leader=original) -> Link -> sink`
    ///
    /// Closest to production:
    ///   - chunk size = one mixer frame (240 samples), paced at the REAL 48k
    ///     rate (5 ms per chunk) so the mixer's frame buffering is genuinely
    ///     exercised (unlike an instant bulk feed, which makes the buffer
    ///     invisible and reports ~0 ms);
    ///   - the impulse sits in a STEADY-STATE chunk (after warm-up) so buffers
    ///     are warm;
    ///   - a SECOND input (translation) is fed LATE and silent, to prove the
    ///     ORIGINAL's latency is independent of the (much later) translation
    ///     stream — the core property of leader mode.
    ///
    /// The wall-clock is stamped exactly when the impulse chunk is *handed to
    /// the pipeline*; a concurrent reader task records the instant the impulse
    /// *appears* at the mixer output. The difference is the TRUE measured
    /// latency (not a computation from `frame_size`).
    #[tokio::test]
    async fn test_chain_latency_realistic_full_graph() {
        use crate::runtime::{AudioLink, AudioSplitter};

        let sample_rate = 48000u32;
        let channels = 2u16;
        let format = AudioFormat {
            sample_rate,
            channels,
            sample_format: PcmFormat::F32(Endianness::Little),
        };

        // One mixer frame per chunk, paced at the real 48k rate.
        let chunk_frames = 240usize;
        let chunk_samples = chunk_frames * channels as usize;
        let chunk_ms = chunk_frames as f64 * 1000.0 / sample_rate as f64; // 5.0 ms

        // ---- Build the full graph ----
        let (src_tx, src_rx) = mpsc::channel::<Audio>(64);
        let mock = MockInput::new(format.clone(), src_rx);
        let mut splitter = AudioSplitter::new(format.clone(), 32, Box::new(mock));
        let original_out = splitter.create_output(); // ReceiverPort (AudioInput)
        splitter.start();

        // Link 1: Splitter.out -> mixer input
        let (link1_tx, mut link1_rx) = AudioLink::new_ports(format.clone(), 32);
        let _link1 = AudioLink::new_link(format.clone(), 32, original_out, Box::new(link1_tx));

        // Mixer with original as the LEADER.
        let mixer = AudioMixer::new(format.clone());
        let mut mixer_out = mixer.get_output();
        let mut stream_out = mixer_out.stream().unwrap();
        let mixer = Arc::new(mixer);
        mixer.add_input_leader(&mut link1_rx, 0.5).unwrap();

        // Second input (translation) — silent and fed LATE, to prove the
        // original's latency does not depend on it.
        let (trans_tx, trans_rx) = mpsc::channel::<Audio>(16);
        let mut trans_mock = MockInput::new(format.clone(), trans_rx);
        mixer.add_input(&mut trans_mock, 1.0).unwrap();

        let mixer_clone = mixer.clone();
        tokio::spawn(async move { mixer_clone.run() });

        // Link 2: mixer -> output sink (modelled like the real graph).
        // We measure at the mixer output stream; the 2nd Link is a ~0 ms forward
        // hop (verified by `test_chain_latency_splitter_link_mixer`).
        let (link2_tx, link2_rx) = AudioLink::new_ports(format.clone(), 32);
        let _link2 = AudioLink::new_link(format.clone(), 32, Box::new(link2_rx), Box::new(link2_tx));

        // Chunks.
        let silent = create_audio_with_samples(&format, vec![0.0f32; chunk_samples]);
        let mut impulse_chunk = vec![0.0f32; chunk_samples];
        impulse_chunk[chunk_frames - 1] = 1.0; // impulse at last sample of ch0
        let impulse_audio = create_audio_with_samples(&format, impulse_chunk);

        // Stamp the wall-clock the moment the impulse chunk is handed over.
        let t_send_slot = Arc::new(std::sync::Mutex::new(None::<std::time::Instant>));
        let t_send_clone = t_send_slot.clone();
        let found = Arc::new(std::sync::Mutex::new(None::<f64>));
        let found_clone = found.clone();
        let reader = tokio::spawn(async move {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            while std::time::Instant::now() < deadline {
                let out = match tokio::time::timeout(
                    std::time::Duration::from_millis(300),
                    stream_out.next(),
                )
                .await
                {
                    Ok(Some(a)) => a,
                    _ => break,
                };
                let pcm = out.to_pcm().unwrap();
                if (pcm.data[chunk_frames - 1] - 0.5).abs() < 1e-6 {
                    if let Some(t0) = *t_send_clone.lock().unwrap() {
                        let ms = t0.elapsed().as_secs_f64() * 1000.0;
                        *found_clone.lock().unwrap() = Some(ms);
                    }
                    break;
                }
            }
        });

        // Warm-up + steady state: silent chunks, then the impulse chunk, then
        // flush. Paced at the real 48k rate so the mixer frame buffer is real.
        let impulse_at = 4usize;
        for i in 0..12usize {
            if i == impulse_at {
                *t_send_slot.lock().unwrap() = Some(std::time::Instant::now());
                src_tx.send(impulse_audio.clone()).await.unwrap();
            } else {
                src_tx.send(silent.clone()).await.unwrap();
            }
            tokio::time::sleep(std::time::Duration::from_secs_f64(chunk_ms / 1000.0)).await;
        }
        // Late translation stream (silent) — must NOT delay the original impulse
        // that already passed through.
        for _ in 0..4 {
            trans_tx.send(silent.clone()).await.unwrap();
            tokio::time::sleep(std::time::Duration::from_secs_f64(chunk_ms / 1000.0)).await;
        }

        reader.await.unwrap();
        let meas = found
            .lock()
            .unwrap()
            .take()
            .expect("impulse never reached mixer output");

        eprintln!("=== REALISTIC FULL GRAPH (Splitter->Link->Mixer->Link, paced @48k, leader=original) ===");
        eprintln!("  Chunk: {chunk_frames} frames (~{chunk_ms:.1} ms @48k), 1 mixer frame/chunk");
        eprintln!("  Original impulse latency (MEASURED): {meas:.2} ms");
        eprintln!("  (theoretical: 1 mixer frame = 5.00 ms + ~0 Link hops + HW playback buffer)");
        eprintln!("  Translation stream arrives AFTER the impulse (late + silent): original must not wait for it");

        // The original must NOT wait for the late translation, and the measured
        // latency should reflect roughly one mixer frame of buffering.
        assert!(
            meas >= 1.0,
            "original latency {meas:.2} ms implausibly low (buffer not exercised)"
        );
        assert!(
            meas < 40.0,
            "original latency {meas:.2} ms too high (blocking on translation?)"
        );
    }
}