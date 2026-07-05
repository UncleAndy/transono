use anyhow::Result;
use tokio::sync::mpsc;
use crate::audio::{Audio, AudioBuffer, AudioCapture, AudioDevices, AudioPipeline, AudioPlayback, FrameConsumer, FrameProducer, RubatoResampler, FRAME_CAPACITY};
use crate::core::provider::Provider;
use crate::providers::openai::realtime::{
    OpenAIRealtimeConfig,
    OpenAIRealtimeProvider,
    RealtimeSession,
    TurnMode,
};
use crate::audio::{
    base64_to_pcm16,
    pcm16_to_base64,
};
use crate::providers::openai::realtime::commands::{
    InputAudioBufferAppend,
};
use cpal::{
    traits::{DeviceTrait, StreamTrait},
    BufferSize, Device, SampleFormat, Stream, StreamConfig,
};
use tokio::task::JoinHandle;
use crate::providers::openai::realtime::events::ProtocolEvent;
use crate::runtime::LineState;

const CAPTURE_BUFFER_SIZE: usize = 256;
const PLAYBACK_BUFFER_SIZE: usize = 256;

pub struct TranslationLine<P: Provider> {
    provider: P,

    capture: AudioCapture,
    playback: AudioPlayback,

    input_pipeline: AudioPipeline,
    output_pipeline: AudioPipeline,

    audio_tx: mpsc::Sender<Audio>,
    audio_rx: Option<mpsc::Receiver<Audio>>,

    capture_thread: Option<JoinHandle<()>>,
    session_task: Option<JoinHandle<Result<()>>>,

    state: LineState,
}

impl<P: Provider> TranslationLine<P> {
    pub async fn new(
        provider: P,
        input: Device,
        output: Device,
    ) -> Result<Self> {
        // Канал между capture-потоком и async-задачей.
        let (audio_tx, audio_rx) =
            mpsc::channel::<Audio>(32);

        // Lock-free буфер микрофона.
        let (capture_tx, capture_rx) =
            AudioBuffer::new(CAPTURE_BUFFER_SIZE)?;

        // Lock-free буфер воспроизведения.
        let (playback_tx, playback_rx) =
            AudioBuffer::new(PLAYBACK_BUFFER_SIZE)?;

        // Создаём устройства.
        let capture =
            AudioCapture::new(
                input,
                capture_tx,
            )?;

        let playback =
            AudioPlayback::new(
                output,
                playback_rx,
            )?;

        Ok(Self {
            provider,

            capture,
            playback,

            capture_rx,
            playback_tx,

            audio_tx,
            audio_rx: Some(audio_rx),

            capture_thread: None,
            session_task: None,

            state: LineState::Created,
        })
    }
    pub async fn run(
        &mut self,
    ) -> Result<()> {

        if self.state == LineState::Running {
            return Ok(());
        }

        // Открываем новую provider session.
        let session =
            self.provider.create_session().await
                .map_err(|e| e.into())?;

        // Забираем Receiver.
        let audio_rx =
            self.audio_rx
                .take()
                .expect("TranslationLine already running");

        // Клон Sender понадобится capture-потоку.
        let capture_tx =
            self.audio_tx.clone();

        // Запускаем аудио.
        self.capture.start()?;
        self.playback.start()?;

        // Запускаем поток чтения микрофона.
        let capture_rx = self.capture_rx;

        self.capture_thread = Some(
            std::thread::spawn(move || {
                capture_forwarder(
                    capture_rx,
                    capture_tx,
                );
            })
        );

        // Забираем producer.
        let playback_tx = self.playback_tx;

        // Запускаем async-задачу общения с Provider.
        self.session_task = Some(
            tokio::spawn(
                realtime_task(
                    session,
                    audio_rx,
                    playback_tx,
                )
            )
        );

        self.state = LineState::Running;

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        todo!()
    }

    pub fn state(&self) -> LineState {
        todo!()
    }
}

fn capture_forwarder(
    mut capture: FrameConsumer,
    tx: Sender<Audio>,
) {
    loop {
        if tx.is_closed() {
            break;
        }

        if let Some(id) = capture.receive() {

            let samples =
                capture.read(id, |frame| {
                    frame.samples().to_vec()
                });

            if tx.blocking_send(samples).is_err() {
                break;
            }

            capture.release(id).unwrap();
        } else {
            std::thread::yield_now();
        }
    }
}

async fn realtime_task(
    mut session: RealtimeSession,
    mut audio_rx: Receiver<Vec<f32>>,
    mut playback: FrameProducer,
) -> Result<()> {

    let mut input_resampler = RubatoResampler::new()?;
    let mut output_resampler = RubatoResampler::new()?;

    let mut input_pcm = Vec::<i16>::new();
    let mut output_float = Vec::<f32>::new();

    loop {

        tokio::select! {

            Some(samples) = audio_rx.recv() => {
                input_pcm.clear();

                input_resampler.in_processor(
                    &samples,
                    &mut input_pcm,
                )?;

                if !input_pcm.is_empty() {
                    let base64 = pcm16_to_base64(&input_pcm);

                    session.send(
                        InputAudioBufferAppend::new(base64)
                    ).await?;
                }
            }

            event = session.next_event() => {

                match event? {

                    ProtocolEvent::ResponseOutputAudioDelta { delta } => {
                        let pcm16 =
                            base64_to_pcm16(&delta)?;

                        output_float.clear();

                        output_resampler.out_processor(
                            &pcm16,
                            &mut output_float,
                        )?;

                        for chunk in output_float.chunks(FRAME_CAPACITY) {
                            playback.send(chunk);
                        }
                    }

                    ProtocolEvent::InputAudioBufferSpeechStarted => {
                        println!("🎤 Speech");
                    }

                    ProtocolEvent::ResponseCreated => {
                        println!("🤖 Thinking...");
                    }

                    ProtocolEvent::ResponseDone => {
                        println!("✅ Done");
                    }

                    ProtocolEvent::Error { error } => {
                        println!("OpenAI: {}", error.message);
                    }

                    _ => {}
                }
            }
        }
    }
}
