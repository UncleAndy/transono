use rubato::Resampler as _;
use rubato::{
    Fft,
    FixedSync,
    audioadapter_buffers::owned::SequentialOwned};
use rubato::audioadapter::{Adapter, AdapterMut};
use symphonia::core::audio::AudioSpec;

use crate::audio::{DspProcessor, PcmAudio};
use crate::core::error::{CoreError, Result};

const SUB_CHUNKS: usize = 4;
const CHUNK_DURATION_MS: u32 = 20;

/// An audio resampler that changes the sample rate.
///
/// Uses FFT-based resampling for high quality conversion. Supports arbitrary
/// input rates and fixed output rates with multi-channel support.
///
/// The resampler accumulates input frames internally and emits output in
/// fixed-size chunks (rubato's `Fft` with `FixedSync::Input`). This is the
/// correct usage pattern: feed as many frames as available, and pull output
/// until none remains. Leftover input (< one chunk) stays buffered for the
/// next call.
pub struct Resampler {
    output_rate: u32,
    fft: Fft<f32>,
    channels: usize,
    chunk_in: usize,
    fft_input: SequentialOwned<f32>,
    fft_output: SequentialOwned<f32>,
    /// Accumulated input frames per channel (between process() calls).
    input_acc: Vec<Vec<f32>>,
    /// Accumulated output frames per channel, ready to be popped.
    output_acc: Vec<Vec<f32>>,
}

impl Resampler {
    /// Creates a new [`Resampler`] with the specified input and output formats.
    pub fn new(
        input_spec: AudioSpec,
        output_rate: u32,
    ) -> Result<Self> {
        let channels = input_spec.channels().count();

        let chunk_size = frames_from_duration(input_spec.rate(), CHUNK_DURATION_MS);

        let fft = Fft::<f32>::new(
            input_spec.rate() as usize,
            output_rate as usize,
            chunk_size,
            SUB_CHUNKS,
            channels,
            FixedSync::Input,
        )
        .map_err(|e| CoreError::Internal(e.to_string()))?;

        let chunk_in = fft.input_frames_max();
        let out_max = fft.output_frames_max();

        Ok(Self {
            output_rate,
            fft,
            channels,
            chunk_in,
            fft_input: SequentialOwned::new(0.0, channels, chunk_in),
            fft_output: SequentialOwned::new(0.0, channels, out_max),
            input_acc: (0..channels).map(|_| Vec::with_capacity(chunk_in)).collect(),
            output_acc: (0..channels).map(|_| Vec::new()).collect(),
        })
    }

    fn push_input(
        &mut self,
        pcm: &PcmAudio,
    ) -> Result<()> {
        let channels_count = pcm.channel_count();

        if channels_count != self.channels {
            return Err(CoreError::Internal(format!(
                "Resampler channel count mismatch: input has {}, expected {}",
                channels_count, self.channels
            )));
        }

        let frames = pcm.frames();
        for channel in 0..channels_count {
            self.input_acc[channel].extend_from_slice(pcm.channel(channel));
        }
        // silence unused warning
        let _ = frames;
        Ok(())
    }

    fn process_fft(
        &mut self,
    ) -> Result<()> {
        while self.input_acc[0].len() >= self.chunk_in {
            // Fill fft_input with exactly `chunk_in` frames per channel.
            for channel in 0..self.channels {
                let slice = &self.input_acc[channel][..self.chunk_in];
                self.fft_input
                    .copy_from_slice_to_channel(channel, 0, slice);
                self.input_acc[channel].drain(..self.chunk_in);
            }

            let (_input_frames, output_frames) = self
                .fft
                .process_into_buffer(&self.fft_input, &mut self.fft_output, None)
                .map_err(|e| CoreError::Internal(e.to_string()))?;

            for channel in 0..self.channels {
                let mut buf = vec![0.0f32; output_frames];
                self.fft_output
                    .copy_from_channel_to_slice(channel, 0, &mut buf);
                self.output_acc[channel].extend_from_slice(&buf);
            }
        }

        Ok(())
    }

    fn pop_output(
        &mut self,
        pcm: &mut PcmAudio,
    ) -> Result<bool> {
        let frames = self.output_acc[0].len();

        if frames == 0 {
            return Ok(false);
        }

        let channels = pcm.channel_count();

        if channels != self.channels {
            return Err(CoreError::Internal(format!(
                "Resampler channel count mismatch: output has {}, expected {}",
                channels, self.channels
            )));
        }

        pcm.resize(frames, channels);

        for channel in 0..channels {
            pcm.channel_mut(channel)
                .copy_from_slice(&self.output_acc[channel]);
        }

        for channel in 0..channels {
            self.output_acc[channel].clear();
        }

        if pcm.spec.rate() != self.output_rate {
            pcm.spec = AudioSpec::new(self.output_rate, pcm.spec.channels().clone());
        }

        Ok(true)
    }
}

impl DspProcessor for Resampler {
    fn process(&mut self, input: &mut PcmAudio) -> Result<bool> {
        self.push_input(input)?;

        self.process_fft()?;

        self.pop_output(input)
    }
}

fn frames_from_duration(
    sample_rate: u32,
    duration_ms: u32,
) -> usize {
    (sample_rate as usize * duration_ms as usize) / 1000
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::{PcmAudio};
    use symphonia::core::audio::{AudioSpec, Channels};

    fn make_pcm(rate: u32, channels: usize, frames: usize, fill: impl Fn(usize, usize) -> f32) -> PcmAudio {
        let ch_layout = if channels == 1 { Channels::Discrete(1) } else { Channels::Discrete(2) };
        let mut pcm = PcmAudio::new(AudioSpec::new(rate, ch_layout), frames);
        for c in 0..channels {
            for i in 0..frames {
                pcm.data[c * frames + i] = fill(c, i);
            }
        }
        pcm
    }

    /// Feed the entire `pcm` into the resampler in one shot (as a real chunk
    /// sized >= the resampler's internal window) and return the resampled
    /// output. `Resampler::process` transforms its input argument in place into
    /// the output, so we pass a copy and read it back.
    fn run_resampler_collect(mut resampler: Resampler, pcm: PcmAudio) -> PcmAudio {
        let channels = pcm.channel_count();
        let frames = pcm.frames();
        let mut slice = PcmAudio::new(pcm.spec.clone(), frames);
        slice.data.copy_from_slice(&pcm.data);

        let produced = resampler.process(&mut slice).unwrap();
        if produced {
            slice
        } else {
            // No output produced (input smaller than one processing window).
            PcmAudio::new(
                AudioSpec::new(
                    resampler.output_rate,
                    if channels == 1 { Channels::Discrete(1) } else { Channels::Discrete(2) },
                ),
                0,
            )
        }
    }

    #[test]
    fn test_resampler_planar_stereo_roundtrip() {
        // Resample 48k stereo -> 44.1k -> back to 48k, feeding in small slices.
        // Output must keep channels isolated and approximate the input.
        let frames_in = 4800; // 100 ms @ 48k
        let original = make_pcm(48000, 2, frames_in, |c, _i| {
            if c == 0 { 0.25 } else { -0.25 }
        });

        let down = Resampler::new(original.spec.clone(), 44100).unwrap();
        let up = Resampler::new(AudioSpec::new(44100, Channels::Discrete(2)), 48000).unwrap();

        let mid = run_resampler_collect(down, original);
        let back = run_resampler_collect(up, mid);

        assert_eq!(back.channel_count(), 2);
        let out_frames = back.frames();
        assert!(out_frames > 0, "resampler produced no output");
        // FFT resamplers insert a startup delay (silence) and may not flush the
        // last partial chunk. Check the steady-state middle: channels must stay
        // isolated and the level preserved.
        let start = out_frames / 4;
        let end = out_frames * 3 / 4;
        for i in start..end {
            let l = back.data[i];
            let r = back.data[out_frames + i];
            assert!((l - r).abs() > 1e-3, "channels collapsed at {}: l={} r={}", i, l, r);
            assert!((l - 0.25).abs() < 0.1, "ch0[{}] off: {}", i, l);
            assert!((r + 0.25).abs() < 0.1, "ch1[{}] off: {}", i, r);
        }
    }

    #[test]
    fn test_resampler_mono_roundtrip_levels() {
        let frames_in = 4800;
        let original = make_pcm(48000, 1, frames_in, |_c, _i| 0.5);
        let down = Resampler::new(original.spec.clone(), 44100).unwrap();
        let up = Resampler::new(AudioSpec::new(44100, Channels::Discrete(1)), 48000).unwrap();

        let mid = run_resampler_collect(down, original);
        let back = run_resampler_collect(up, mid);

        let out_frames = back.frames();
        assert!(out_frames > 0, "resampler produced no output");
        // FFT resamplers insert a startup delay (silence) at the beginning and
        // may not flush the very last partial chunk. Check the steady-state
        // middle of the signal, where the level must be preserved.
        let start = out_frames / 4;
        let end = out_frames * 3 / 4;
        for i in start..end {
            assert!((back.data[i] - 0.5).abs() < 0.1, "sample {} off: {}", i, back.data[i]);
        }
    }
}
