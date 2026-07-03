use anyhow::Result;

pub struct RealtimeClient {
    api_key: String,
}

impl RealtimeClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        // Здесь будет подключение через WebSocket.
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        Ok(())
    }

    pub async fn send_audio(
        &mut self,
        _pcm16: &[i16],
    ) -> Result<()> {
        Ok(())
    }

    pub async fn receive_audio(
        &mut self,
    ) -> Result<Option<Vec<i16>>> {
        Ok(None)
    }
}
