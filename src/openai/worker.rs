use anyhow::Result;
use rtrb::{Consumer, RingBuffer};
use std::fs::File;
use std::io::BufWriter;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use hound::{SampleFormat, WavSpec, WavWriter};

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

                let mut wav: Option<WavWriter<BufWriter<File>>> = None;
                let mut wav_index = 0usize;

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

                                            if wav.is_none() {
                                                let spec = WavSpec {
                                                    channels: 1,
                                                    sample_rate: 24_000,
                                                    bits_per_sample: 16,
                                                    sample_format: SampleFormat::Int,
                                                };

                                                let filename = format!("openai_{wav_index}.wav");

                                                println!("Recording {filename}");

                                                wav = Some(
                                                    WavWriter::create(filename, spec)
                                                        .expect("create wav"),
                                                );
                                            }

                                            if let Some(writer) = wav.as_mut() {
                                                for &sample in &chunk {
                                                    writer.write_sample(sample).ok();
                                                }
                                            }

                                            let _ = output_tx.push(chunk);
                                        }

                                        Err(err) => {
                                            eprintln!("decode: {err}");
                                        }
                                    }
                                }

                                Ok(crate::openai::events::ServerEvent::ResponseOutputAudioDone) => {
                                    // println!("ResponseOutputAudioDone");
                                    if let Some(writer) = wav.take() {
                                        writer.finalize().ok();
                                        println!("WAV saved");
                                        wav_index += 1;
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
