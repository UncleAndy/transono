use crate::audio::processor::AudioProcessor;
use anyhow::Result;
use crate::audio::Audio;

pub struct AudioPipeline {
    processors: Vec<Box<dyn AudioProcessor>>,
}

impl AudioPipeline {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
        }
    }

    pub fn add<P>(
        &mut self,
        processor: P,
    ) -> &mut Self
    where
        P: AudioProcessor + 'static,
    {
        self.processors.push(Box::new(processor));
        self
    }

    pub fn process(
        &mut self,
        mut audio: Audio,
    ) -> Result<Audio> {

        for processor in &mut self.processors {
            audio = processor.process(audio)?;
        }

        Ok(audio)
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
