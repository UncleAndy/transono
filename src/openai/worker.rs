use anyhow::Result;
use tokio::runtime::Runtime;

use crate::openai::realtime::RealtimeClient;
use crate::audio::processor::AudioProcessor;

pub struct OpenAiWorker {
    rt: Runtime,
    client: RealtimeClient,
}

impl OpenAiWorker {
    pub fn connect(
        api_key: &str,
        instructions: &str,
    ) -> Result<Self> {
        let rt = Runtime::new()?;

        let client = rt.block_on(async {
            RealtimeClient::connect(api_key, instructions).await
        })?;

        Ok(Self {
            rt,
            client,
        })
    }

    #[inline]
    pub fn append_audio(
        &mut self,
        pcm: &[i16],
    ) -> Result<()> {
        self.rt
            .block_on(self.client.append_audio(pcm))
    }

    #[inline]
    pub fn commit(&mut self) -> Result<()> {
        self.rt
            .block_on(self.client.commit_audio())
    }

    #[inline]
    pub fn create_response(&mut self) -> Result<()> {
        self.rt
            .block_on(self.client.create_response())
    }

    #[inline]
    pub fn next_audio(
        &mut self,
    ) -> Result<Option<Vec<i16>>> {
        self.rt
            .block_on(self.client.next_audio())
    }
}

impl AudioProcessor for OpenAiWorker {
    fn push_audio(
        &mut self,
        input: &[i16],
    ) -> Result<()> {
        self.append_audio(input)?;
        self.commit()?;
        self.create_response()?;
        Ok(())
    }

    fn poll_audio(
        &mut self,
    ) -> Result<Option<Vec<i16>>> {
        self.next_audio()
    }
}
