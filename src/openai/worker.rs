use anyhow::Result;
use rtrb::{Consumer, Producer, RingBuffer};
use tokio::runtime::Runtime;

use crate::{
    audio::processor::AudioProcessor,
    openai::realtime::RealtimeClient,
};

const QUEUE_SIZE: usize = 64;

pub struct OpenAiWorker {
    input: Producer<Vec<i16>>,
    output: Consumer<Vec<i16>>,
}

impl OpenAiWorker {
    pub fn connect(
        api_key: &str,
        instructions: &str,
    ) -> Result<Self> {
        let (input_tx, input_rx) =
            RingBuffer::<Vec<i16>>::new(QUEUE_SIZE);

        let (output_tx, output_rx) =
            RingBuffer::<Vec<i16>>::new(QUEUE_SIZE);

        let api_key = api_key.to_owned();
        let instructions = instructions.to_owned();

        std::thread::spawn(move || {
            let rt = Runtime::new().unwrap();

            rt.block_on(async move {
                let mut client = match RealtimeClient::connect(
                    &api_key,
                    &instructions,
                )
                    .await
                {
                    Ok(client) => client,
                    Err(err) => {
                        eprintln!("Realtime: {err}");
                        return;
                    }
                };

                //
                // Пока оставляем заглушку.
                // Следующим коммитом сюда переедет
                // вся работа с WebSocket.
                //
                let _ = (
                    client,
                    input_rx,
                    output_tx,
                );

                futures::future::pending::<()>().await;
            });
        });

        Ok(Self {
            input: input_tx,
            output: output_rx,
        })
    }
}

impl AudioProcessor for OpenAiWorker {
    fn push_audio(
        &mut self,
        input: &[i16],
    ) -> Result<()> {
        self.input
            .push(input.to_vec())
            .map_err(|_| anyhow::anyhow!("OpenAI input queue overflow"))?;

        Ok(())
    }
    
    fn poll_audio(
        &mut self,
    ) -> Result<Option<Vec<i16>>> {
        match self.output.pop() {
            Ok(chunk) => Ok(Some(chunk)),
            Err(rtrb::PopError::Empty) => Ok(None),
        }
    }
}
