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
        let (input_tx, mut input_rx) =
            RingBuffer::<Vec<i16>>::new(QUEUE_SIZE);

        let (mut output_tx, output_rx) =
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

                loop {
                    //
                    // Отправляем накопленный звук.
                    //
                    while let Ok(audio) = input_rx.pop() {
                        if let Err(err) = client.append_audio(&audio).await {
                            eprintln!("append_audio: {err}");
                            continue;
                        }

                        if let Err(err) = client.commit_audio().await {
                            eprintln!("commit_audio: {err}");
                            continue;
                        }

                        if let Err(err) = client.create_response().await {
                            eprintln!("create_response: {err}");
                        }
                    }

                    //
                    // Забираем всё готовое аудио.
                    //
                    loop {
                        match client.next_audio().await {
                            Ok(Some(chunk)) => {
                                let _ = output_tx.push(chunk);
                            }

                            Ok(None) => {
                                break;
                            }

                            Err(err) => {
                                eprintln!("next_audio: {err}");
                                break;
                            }
                        }
                    }

                    tokio::task::yield_now().await;
                }
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
