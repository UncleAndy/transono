//! Integration tests for the rtrb-backed audio graph (step 2.1 switch).
//!
//! These assemble the *real* topology the binary now uses — splitter fan-out
//! on rtrb rings, `AudioLink::new_ports_rtrb`/`new_link_rtrb` hops, and the
//! mixer's `get_output_rtrb`/`run_rtrb` output path — without any hardware,
//! and assert that audio actually flows end-to-end with correct samples and
//! that the leader semantics survive the channel-engine swap.

use crate::audio::{Audio, AudioFormat, AudioInput, AudioOutput, PcmAudio};
use crate::core::error::{CoreError, Result};
use crate::runtime::{AudioLink, AudioMixer, AudioSplitter, ReceiverPort, SenderPort};

use futures_util::stream::BoxStream;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
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

/// rtrb splitter must broadcast identical frames to every rtrb fan-out.
#[tokio::test]
async fn splitter_rtrb_broadcasts_to_all_outputs() {
    let fmt = test_format();
    let (hw_tx, hw_rx) = mpsc::channel(64);
    let mut splitter = AudioSplitter::new(fmt.clone(), 32, Box::new(MockInput::new(fmt.clone(), hw_rx)));
    let mut out1 = splitter.create_output_rtrb();
    let mut out2 = splitter.create_output_rtrb();
    splitter.start_rtrb();

    let frames = 16usize;
    let ch = fmt.channels as usize;
    let mut data = vec![0.0f32; frames * ch];
    for i in 0..frames {
        data[i] = 0.3; // channel 0
        if ch > 1 {
            data[frames + i] = 0.7; // channel 1
        }
    }
    let audio = audio_with_samples(&fmt, data.clone());
    hw_tx.send(audio.clone()).await.unwrap();

    let mut s1 = out1.stream().unwrap();
    let mut s2 = out2.stream().unwrap();
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
    assert_eq!(p1.data.len(), data.len(), "splitter changed frame size");
    assert_eq!(p1.data, p2.data, "rtrb fan-outs diverged");
    assert_eq!(p1.data, data, "splitter altered sample");
}

/// A single `new_ports_rtrb` + `new_link_rtrb` hop forwards audio unchanged.
#[tokio::test]
async fn link_rtrb_forwards_between_ports() {
    let fmt = test_format();
    let (mut tx, rx) = AudioLink::new_ports_rtrb(fmt.clone(), 8);

    let (collect_tx, collect_rx) = mpsc::channel(8);
    let out = SenderPort::new(fmt.clone(), collect_tx);
    let _link = AudioLink::new_link_rtrb(fmt.clone(), 8, Box::new(rx), Box::new(out));

    let frames = 12usize;
    let ch = fmt.channels as usize;
    let data = vec![0.42f32; frames * ch];
    let audio = audio_with_samples(&fmt, data.clone());

    {
        let mut sink = tx.sink().unwrap();
        sink.send(audio.clone()).await.unwrap();
    }

    let mut cp = ReceiverPort::new(fmt.clone(), collect_rx);
    let mut cs = cp.stream().unwrap();
    let got = tokio::time::timeout(Duration::from_secs(1), cs.next())
        .await
        .expect("collect empty")
        .expect("collect ended");
    assert_eq!(got.to_pcm().unwrap().data, data, "link_rtrb altered sample");
}

/// Full graph (no hardware): hw -> splitter(rtrb, 2 outs) ->
/// [original -> mixer(leader, rtrb)] -> mixer_out(rtrb) -> link_rtrb -> virt-mic.
/// Asserts the original reaches the virtual microphone with correct samples.
#[tokio::test]
async fn full_graph_rtrb_original_reaches_virt_mic() {
    let fmt = test_format();
    let ch = fmt.channels as usize;

    let (hw_tx, hw_rx) = mpsc::channel(64);
    let mut splitter = AudioSplitter::new(fmt.clone(), 32, Box::new(MockInput::new(fmt.clone(), hw_rx)));
    let out_translate = splitter.create_output_rtrb();
    let mut out_original = splitter.create_output_rtrb();

    let mixer = AudioMixer::new(fmt.clone());
    let (to_mixer_tx, mut to_mixer_rx) = AudioLink::new_ports_rtrb(fmt.clone(), 32);
    // translation channel (carries the same splitter output in this test).
    mixer.add_input(&mut to_mixer_rx, 1.0).unwrap();
    // original is the LEADER.
    mixer.add_input_leader(out_original.as_mut(), 0.5).unwrap();
    let mixer_out = mixer.get_output_rtrb();

    // virt-mic sink.
    let (virt_tx, virt_rx) = mpsc::channel(32);
    let virt_out = SenderPort::new(fmt.clone(), virt_tx);
    let _link_tr = AudioLink::new_link_rtrb(fmt.clone(), 32, out_translate, Box::new(to_mixer_tx));
    let _link_virt = AudioLink::new_link_rtrb(fmt.clone(), 32, Box::new(mixer_out), Box::new(virt_out));

    splitter.start_rtrb();
    let mixer = Arc::new(mixer);
    mixer.clone().run_rtrb();

    // Feed a steady stream so buffers stay warm.
    for _ in 0..6 {
        let data = vec![0.6f32; 240 * ch];
        hw_tx.send(audio_with_samples(&fmt, data)).await.unwrap();
    }

    let mut virt_port = ReceiverPort::new(fmt.clone(), virt_rx);
    let mut vstream = virt_port.stream().unwrap();

    let mut got_frames = 0usize;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline && got_frames < 3 {
        let next = tokio::time::timeout(Duration::from_millis(300), vstream.next()).await;
        match next {
            Ok(Some(a)) => {
                let pcm = a.to_pcm().unwrap();
                // Original weight 0.5 * 0.6 = 0.3 expected (translation also 0.6 at weight 1.0
                // => total 0.9, clamped stays 0.9). Either way every sample must be in range.
                for &s in pcm.data.iter() {
                    assert!(s.abs() <= 1.0 + 1e-6, "sample out of range: {s}");
                    assert!((s - 0.3).abs() < 1e-6 || (s - 0.9).abs() < 1e-6, "unexpected mix: {s}");
                }
                got_frames += 1;
            }
            _ => break,
        }
    }
    assert!(got_frames >= 3, "virt-mic received too few frames via rtrb graph");
}

/// The original must flow to the virtual mic promptly and NOT wait for a late,
/// silent translation stream — the core leader property, verified on the rtrb
/// graph topology (splitter -> mixer(leader) -> link -> virt-mic + a separate
/// late translation link into the mixer).
#[tokio::test]
async fn full_graph_rtrb_original_independent_of_late_translation() {
    let fmt = test_format();
    let ch = fmt.channels as usize;

    // Original path: real mic -> splitter -> original out (leader).
    let (hw_tx, hw_rx) = mpsc::channel(64);
    let mut splitter =
        AudioSplitter::new(fmt.clone(), 32, Box::new(MockInput::new(fmt.clone(), hw_rx)));
    let mut out_original = splitter.create_output_rtrb();

    // Translation path: a SEPARATE feed into the mixer (models the OpenAI
    // translation that, in production, arrives hundreds of ms later). Here the
    // translation input is left EMPTY while we check the original reaches the
    // mic — proving the original never waits for it.
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

    let chunk_frames = 240usize;
    let silent = audio_with_samples(&fmt, vec![0.0f32; chunk_frames * ch]);
    let mut impulse = vec![0.0f32; chunk_frames * ch];
    impulse[chunk_frames - 1] = 1.0; // impulse at last sample of ch0
    let impulse_audio = audio_with_samples(&fmt, impulse);

    // Feed the ORIGINAL first (impulse + steady silent), no translation yet.
    for i in 0..12usize {
        if i == 4 {
            hw_tx.send(impulse_audio.clone()).await.unwrap();
        } else {
            hw_tx.send(silent.clone()).await.unwrap();
        }
    }

    // Read the virtual-mic output and confirm the original impulse arrived.
    let mut virt_port = ReceiverPort::new(fmt.clone(), virt_rx);
    let mut vstream = virt_port.stream().unwrap();
    let mut original_impulse_seen = false;
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        let next = tokio::time::timeout(Duration::from_millis(300), vstream.next()).await;
        match next {
            Ok(Some(a)) => {
                let pcm = a.to_pcm().unwrap();
                // ch0 last sample ~0.5 (original 1.0 * weight 0.5); translation
                // is still silent here, so nothing else contributes.
                if (pcm.data[chunk_frames - 1] - 0.5).abs() < 1e-6 {
                    original_impulse_seen = true;
                    break;
                }
            }
            _ => break,
        }
    }
    assert!(
        original_impulse_seen,
        "original impulse did not reach virt-mic while translation was still empty \
         (leader semantics broken on rtrb graph)"
    );
}
