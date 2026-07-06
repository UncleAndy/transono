use crate::audio::processor::AudioProcessor;
use anyhow::Result;
use crate::audio::{Audio, AudioFormat, RubatoResampler};

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

    pub fn prepare_input(
        &mut self,
        from: &AudioFormat,
        to: &AudioFormat,
    ) -> Result<()> {
        self.prepare(from, to)
    }

    pub fn prepare_output(
        &mut self,
        from: &AudioFormat,
        to: &AudioFormat,
    ) -> Result<()> {
        self.prepare(from, to)
    }

    pub fn prepare(
        &mut self,
        from: &AudioFormat,
        to: &AudioFormat,
    ) -> Result<()> {

        self.clear();

        if from.channels != to.channels {
            self.add(ChannelConverter::new(
                from.channels,
                to.channels,
            ));
        }

        if from.sample_rate != to.sample_rate {
            self.add(RubatoResampler::new(
                from.sample_rate,
                to.sample_rate,
            )?);
        }

        Ok(())
    }
}

impl Default for AudioPipeline {
    fn default() -> Self {
        Self::new()
    }
}
