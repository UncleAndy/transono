use crate::audio::{Audio, PcmAudio, Processor};
use crate::core::error::{CoreError, Result};

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

    pub fn add<P>(
        &mut self,
        processor: Processor,
    ) -> &mut Self
    where
        P: 'static,
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
                    match data {
                        PipelineState::Audio(audio) => proc.process(audio),
                        PipelineState::Pcm(_) => { return Err(CoreError::Other(anyhow::Error::msg("can not use Pcm for Audio processor"))) },
                    }
                }
                Processor::Dsp(proc) => {
                    match data {
                        PipelineState::Pcm(audio) => proc.process(audio),
                        PipelineState::Audio(_) => { return Err(CoreError::Other(anyhow::Error::msg("can not use Audio for Dsp processor"))) },
                    }
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
