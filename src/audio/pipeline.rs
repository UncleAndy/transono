use crate::audio::{Audio, PcmAudio, Processor};
use crate::core::error::Result;

pub struct AudioPipeline {
    processors: Vec<Processor>,
    scratch_pcm: Option<PcmAudio>,
}

impl AudioPipeline {
    pub fn new() -> Self {
        Self {
            processors: Vec::new(),
            scratch_pcm: None,
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
        audio: Audio,
    ) -> Result<Audio> {
        if self.processors.is_empty() {
            return Ok(audio);
        }

        let mut current_audio = Some(audio);

        for processor in &mut self.processors {
            match processor {
                Processor::Audio(proc) => {
                    let mut audio = if let Some(a) = current_audio.take() {
                        a
                    } else {
                        Audio::from_pcm(self.scratch_pcm.as_ref().expect("scratch_pcm must be initialized"))?
                    };

                    proc.process(&mut audio)?;
                    current_audio = Some(audio);
                }
                Processor::Dsp(dsp) => {
                    if let Some(audio) = current_audio.take() {
                        if let Some(ref mut scratch) = self.scratch_pcm {
                            audio.to_pcm_into(scratch)?;
                        } else {
                            self.scratch_pcm = Some(audio.to_pcm()?);
                        }
                    }
                    dsp.process(self.scratch_pcm.as_mut().expect("scratch_pcm must be initialized"))?;
                }
            }
        }

        if let Some(audio) = current_audio {
            Ok(audio)
        } else {
            Audio::from_pcm(self.scratch_pcm.as_ref().expect("scratch_pcm must be initialized"))
        }
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

pub struct Pipelines {
    pub input: AudioPipeline,
    pub output: AudioPipeline,
}
