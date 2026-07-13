#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{Audio, AudioFormat, PcmAudio, PcmFormat, Endianness, AudioInput};
    use crate::runtime::Mixer;
    use tokio::sync::mpsc;
    use std::sync::Arc;
    use symphonia::core::audio::{AudioSpec, Channels};

    fn create_test_format() -> AudioFormat {
        AudioFormat {
            sample_rate: 48000,
            channels: 2,
            sample_format: PcmFormat::F32(Endianness::Little),
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
            Self { format, receiver: Some(rx) }
        }
    }

    impl crate::audio::AudioInput for MockInput {
        fn take_receiver(&mut self) -> crate::core::error::Result<mpsc::Receiver<Audio>> {
            self.receiver.take().ok_or_else(|| crate::core::error::CoreError::Other(anyhow::anyhow!("receiver taken")))
        }
        fn start(&self) -> crate::core::error::Result<()> { Ok(()) }
        fn stop(&self) -> crate::core::error::Result<()> { Ok(()) }
        fn format(&self) -> &AudioFormat { &self.format }
    }

    #[tokio::test]
    async fn test_mixer_format_compatibility() {
        let format = create_test_format();
        let mixer = Mixer::new(format.clone());

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
        let mut mixer = Mixer::new(format.clone());
        let mut rx_out = mixer.take_receiver().unwrap();
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
        let samples1 = vec![0.5f32; 480 * 2];
        let samples2 = vec![0.2f32; 480 * 2];
        
        tx1.send(create_audio_with_samples(&format, samples1)).await.unwrap();
        tx2.send(create_audio_with_samples(&format, samples2)).await.unwrap();

        let mixer_clone = mixer.clone();
        tokio::spawn(async move {
            mixer_clone.run().await;
        });

        let mixed_audio = rx_out.recv().await.expect("Mixer should produce output");
        let pcm = mixed_audio.to_pcm().unwrap();
        
        for &sample in pcm.data.iter() {
            assert!((sample - 0.6).abs() < 1e-6, "Expected 0.6, got {}", sample);
        }
    }

    #[tokio::test]
    async fn test_clamping() {
        let format = create_test_format();
        let mut mixer = Mixer::new(format.clone());
        let mut rx_out = mixer.take_receiver().unwrap();
        let mixer = Arc::new(mixer);

        let (tx1, rx1) = mpsc::channel(10);
        let mut input1 = MockInput::new(format.clone(), rx1);
        mixer.add_input(&mut input1, 1.0).unwrap();

        let (tx2, rx2) = mpsc::channel(10);
        let mut input2 = MockInput::new(format.clone(), rx2);
        mixer.add_input(&mut input2, 1.0).unwrap();

        // 0.8 + 0.8 = 1.6 -> should clamp to 1.0
        let samples1 = vec![0.8f32; 480 * 2];
        let samples2 = vec![0.8f32; 480 * 2];

        tx1.send(create_audio_with_samples(&format, samples1)).await.unwrap();
        tx2.send(create_audio_with_samples(&format, samples2)).await.unwrap();

        let mixer_clone = mixer.clone();
        tokio::spawn(async move {
            mixer_clone.run().await;
        });

        let mixed_audio = rx_out.recv().await.expect("Mixer should produce output");
        let pcm = mixed_audio.to_pcm().unwrap();
        
        for &sample in pcm.data.iter() {
            assert_eq!(sample, 1.0, "Expected clamped 1.0, got {}", sample);
        }
    }
}
