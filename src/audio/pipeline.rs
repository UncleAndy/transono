use crate::audio::{Audio, PcmAudio, Processor};
use crate::core::error::{Result};

pub struct AudioPipeline {
    processors: Vec<Processor>,
}

enum PipelineState {
    Audio(Audio),
    Pcm(PcmAudio),
}

impl AudioPipeline {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    pub fn add(
        &mut self,
        processor: Processor,
    ) -> &mut Self
    {
        self.processors.push(processor);
        self
    }

    pub fn process(
        &mut self,
        data: &mut PipelineState,
    ) -> Result<()> {

        for processor in &mut self.processors {
            let _ = match processor {
                Processor::Audio(proc) => {
                    proc.process(data.ensure_audio()?)
                }
                Processor::Dsp(dsp) => {
                    dsp.process(data.ensure_pcm()?)
                }
            };
        };

        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        self.processors.is_empty()
    }

    pub fn clear(&mut self) {
        self.processors.clear()
    }
}

impl Default for AudioPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl PipelineState {
    fn ensure_pcm(
        &mut self,
    ) -> Result<&mut PcmAudio> {
        if let PipelineState::Audio(audio) = self {
            let pcm = audio.to_pcm()?;

            *self = PipelineState::Pcm(pcm);
        }

        match self {
            PipelineState::Pcm(pcm) => Ok(pcm),
            _ => unreachable!(),
        }
    }

    fn ensure_audio(
        &mut self,
    ) -> Result<&mut Audio> {

        if let PipelineState::Pcm(pcm) = self {

            let audio = Audio::from_pcm(&pcm)?;

            *self = PipelineState::Audio(audio);
        }

        match self {
            PipelineState::Audio(audio) => {
                Ok(audio)
            }
            _ => unreachable!(),
        }
    }
}
