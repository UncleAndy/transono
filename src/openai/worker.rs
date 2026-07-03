use anyhow::Result;
use rtrb::{Consumer, RingBuffer};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use crate::{audio::processor::AudioProcessor, openai::realtime::RealtimeClient};

const QUEUE_SIZE: usize = 256;

pub struct OpenAiWorker {
    input: mpsc::UnboundedSender<Vec<i16>>,
    output: Consumer<Vec<i16>>,
}

impl OpenAiWorker {
    pub fn connect(api_key: &str, instructions: &str) -> Result<Self> {
        let (input_tx, mut input_rx) = mpsc::unbounded_channel::<Vec<i16>>();

        let (mut output_tx, output_rx) = RingBuffer::<Vec<i16>>::new(QUEUE_SIZE);

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
                    tokio::select! {
                        Some(audio) = input_rx.recv() => {
                            if let Err(err) = client.append_audio(&audio).await {
                                eprintln!("append_audio: {err}");
                            }
                        }

                        event = client.next_event() => {
                            match event {
                                Ok(crate::openai::events::ServerEvent::ResponseOutputAudioDelta { delta }) => {
                                    match crate::openai::audio::base64_to_pcm16(&delta) {
                                        Ok(chunk) => {
                                            let _ = output_tx.push(chunk);
                                        }

                                        Err(err) => {
                                            eprintln!("decode: {err}");
                                        }
                                    }
                                }

                                Ok(_event) => {
                                    // println!("{event:#?}");
                                }

                                Err(err) => {
                                    eprintln!("next_event: {err}");
                                    break;
                                }
                            }
                        }
                    }
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
    fn push_audio(&mut self, input: &[i16]) -> Result<()> {
        self.input
            .send(input.to_vec())
            .map_err(|_| anyhow::anyhow!("OpenAI worker stopped"))
    }

    fn poll_audio(&mut self) -> Result<Option<Vec<i16>>> {
        match self.output.pop() {
            Ok(chunk) => Ok(Some(chunk)),
            Err(rtrb::PopError::Empty) => Ok(None),
        }
    }
}
