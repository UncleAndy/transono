use std::cell::RefCell;
use std::io::Cursor;
use std::rc::Rc;
use std::sync::{mpsc, Arc};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;
use pipewire::context::ContextRc;
use pipewire::core::CoreRc;
use pipewire::{keys, spa};
use pipewire::loop_::Timeout;
use pipewire::main_loop::MainLoopRc;
use pipewire::properties::properties;
use pipewire::spa::pod::{Pod, Value};
use pipewire::spa::pod::serialize::PodSerializer;
use pipewire::stream::{StreamFlags, StreamListener, StreamRc};

use crate::audio::{AudioFormat, FrameConsumer, FrameId, FrameProducer};
use crate::core::error::Result;

pub struct PipeWireWorker {
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    command_tx: mpsc::Sender<WorkerCommand>,
}

enum WorkerCommand {
    Drain(mpsc::Sender<()>),
}

pub struct FrameReader {
    consumer: FrameConsumer,
    current: Option<FrameId>,
    offset: usize,
}

impl FrameReader {
    pub fn fill(&mut self, output: &mut [f32]) {
        self.consumer.fill_buffer(
            &mut self.current,
            &mut self.offset,
            output,
        );
    }
}

struct OutputState {
    reader: FrameReader,
}

struct InputState {
    writer: FrameWriter,
}

enum WorkerState {
    Output(OutputState),
    Input(InputState),
}

struct FrameWriter {
    producer: FrameProducer,
}

impl FrameWriter {
    pub fn write(&mut self, input: &[f32]) {
        let _ = self.producer.send(input);
    }
}

// Эти поля не используются напрямую.
// Они удерживают PipeWire-объекты живыми до конца жизни сессии.
struct PipeWireSession {
    _main_loop: MainLoopRc,
    _context: ContextRc,
    _core: CoreRc,

    _stream: StreamRc,
    _stream_listener: StreamListener<WorkerState>,
    pending_drain: Rc<RefCell<Option<mpsc::Sender<()>>>>,
    command_rx: mpsc::Receiver<WorkerCommand>,

    _format: AudioFormat,
    _node_name: String,

    _pod_bytes: Vec<u8>,
}

pub enum WorkerEndpoint {
    Output(FrameConsumer),
    Input(FrameProducer),
}

pub struct WorkerConfig {
    pub node_name: String,
    pub format: AudioFormat,
    pub endpoint: WorkerEndpoint,

    /// PipeWire node id.
    pub node_id: Option<u32>,
}

impl PipeWireWorker {
    pub fn spawn_output(
        consumer: FrameConsumer,
        format: AudioFormat,
        node_name: String,
        node_id: Option<u32>,
    ) -> Result<Self> {
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = shutdown.clone();

        let (command_tx, command_rx) = mpsc::channel();

        let thread = std::thread::spawn(move || {
            let config = WorkerConfig {
                node_name,
                format,
                endpoint: WorkerEndpoint::Output(consumer),
                node_id,
            };

            if let Err(e) = Self::run(thread_shutdown, command_rx, config) {
                eprintln!("PipeWire worker failed: {e}");
            }
        });

        Ok(Self {
            shutdown,
            command_tx,
            thread: Some(thread),
        })
    }

    pub fn spawn_input(
        producer: FrameProducer,
        format: AudioFormat,
        node_name: String,
        node_id: Option<u32>,
    ) -> Result<Self> {
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = shutdown.clone();

        let (command_tx, command_rx) = mpsc::channel();

        let thread = std::thread::spawn(move || {
            let config = WorkerConfig {
                node_name,
                format,
                endpoint: WorkerEndpoint::Input(producer),
                node_id,
            };

            if let Err(e) = Self::run(thread_shutdown, command_rx, config) {
                eprintln!("PipeWire worker failed: {e}");
            }
        });

        Ok(Self {
            shutdown,
            command_tx,
            thread: Some(thread),
        })
    }

    fn run(
        shutdown: Arc<AtomicBool>,
        command_rx: mpsc::Receiver<WorkerCommand>,
        config: WorkerConfig,
    ) -> Result<()> {
        let ctx = PipeWireSession::new(config, command_rx)?;

        while !shutdown.load(Ordering::Acquire) {
            ctx.process_commands()?;

            ctx._main_loop
                .loop_()
                .iterate(
                    Timeout::Finite(Duration::from_millis(10))
                );

            if shutdown.load(Ordering::Acquire) {
                break;
            }
        }

        Ok(())
    }

    pub fn shutdown(&mut self) -> Result<()> {
        // Доигрываем все данные.
        self.drain()?;

        // Просим поток завершиться.
        self.shutdown.store(true, Ordering::Release);

        // Ждем завершения потока.
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }

        Ok(())
    }

    pub fn drain(&self) -> Result<()> {
        let (tx, rx) = mpsc::channel();

        let _ = self.command_tx
            .send(WorkerCommand::Drain(tx));
        let _ = rx.recv();

        Ok(())
    }
}

impl PipeWireSession {
    fn create_audio_params(
        format: &AudioFormat,
    ) -> Result<Vec<u8>> {
        let object = spa::pod::object!(
            spa::utils::SpaTypes::ObjectParamFormat,
            spa::param::ParamType::EnumFormat,

            spa::pod::property!(
                spa::param::format::FormatProperties::MediaType,
                Id,
                spa::param::format::MediaType::Audio
            ),

            spa::pod::property!(
                spa::param::format::FormatProperties::MediaSubtype,
                Id,
                spa::param::format::MediaSubtype::Raw
            ),

            spa::pod::property!(
                spa::param::format::FormatProperties::AudioFormat,
                Id,
                spa::param::audio::AudioFormat::F32LE
            ),

            spa::pod::property!(
                spa::param::format::FormatProperties::AudioRate,
                Int,
                format.sample_rate as i32
            ),

            spa::pod::property!(
                spa::param::format::FormatProperties::AudioChannels,
                Int,
                format.channels as i32
            ),
        );

        let bytes = PodSerializer::serialize(
            Cursor::new(Vec::new()),
            &Value::Object(object),
        )?
            .0
            .into_inner();

        Ok(bytes)
    }

    pub fn new(config: WorkerConfig, command_rx: mpsc::Receiver<WorkerCommand>) -> Result<Self> {
        pipewire::init();

        let main_loop = MainLoopRc::new(None)?;
        let context = ContextRc::new(&main_loop, None)?;
        let core = context.connect_rc(None)?;

        let direction = match &config.endpoint {
            WorkerEndpoint::Output(_) => spa::utils::Direction::Output,
            WorkerEndpoint::Input(_) => spa::utils::Direction::Input,
        };

        let media_category = match &config.endpoint {
            WorkerEndpoint::Output(_) => "Playback",
            WorkerEndpoint::Input(_) => "Capture",
        };

        let properties = properties! {
            *keys::MEDIA_TYPE        => "Audio",
            *keys::MEDIA_CATEGORY    => media_category,
            *keys::MEDIA_ROLE        => "Communication",
            *keys::NODE_NAME         => config.node_name.clone(),
            *keys::NODE_DESCRIPTION  => config.node_name.clone(),
        };

        let stream = StreamRc::new(
            core.clone(),
            &config.node_name,
            properties,
        )?;

        let frame_stride =
            config.format.frame_size() as i32;

        let pending_drain: Rc<RefCell<Option<mpsc::Sender<()>>>> =
            Rc::new(RefCell::new(None));

        let stream_listener = match config.endpoint {
            WorkerEndpoint::Output(consumer) => {
                let pending_drain_cb =
                    pending_drain.clone();

                stream
                    .add_local_listener_with_user_data(
                        WorkerState::Output(
                            OutputState {
                                reader: FrameReader {
                                    consumer,
                                    current: None,
                                    offset: 0,
                                },
                            }
                        )
                    )
                    .state_changed(|stream, _, old, new| {
                        println!(
                            "{}: {:?} -> {:?}",
                            stream.name(),
                            old,
                            new
                        );
                    })
                    .process(move |stream, state| {
                        let WorkerState::Output(state) = state else {
                            return;
                        };

                        let Some(mut buffer) =
                            stream.dequeue_buffer()
                        else {
                            return;
                        };

                        let datas = buffer.datas_mut();

                        if datas.is_empty() {
                            return;
                        }

                        let data = &mut datas[0];

                        let size = {
                            let Some(bytes) = data.data() else {
                                return;
                            };

                            let samples: &mut [f32] = unsafe {
                                std::slice::from_raw_parts_mut(
                                    bytes.as_mut_ptr() as *mut f32,
                                    bytes.len()
                                        / size_of::<f32>(),
                                )
                            };

                            state.reader.fill(samples);

                            bytes.len()
                        };

                        let chunk = data.chunk_mut();

                        *chunk.offset_mut() = 0;
                        *chunk.stride_mut() = frame_stride;
                        *chunk.size_mut() = size as u32;
                    })
                    .drained(move |_, _| {
                        if let Some(done) =
                            pending_drain_cb.borrow_mut().take()
                        {
                            let _ = done.send(());
                        }
                    })
                    .register()?
            }
            WorkerEndpoint::Input(producer) => {
                stream
                    .add_local_listener_with_user_data(
                        WorkerState::Input(
                            InputState {
                                writer: FrameWriter {
                                    producer,
                                }
                            }
                        )
                    )
                    .state_changed(|stream, _, old, new| {
                        println!(
                            "{}: {:?} -> {:?}",
                            stream.name(),
                            old,
                            new
                        );
                    })
                    .process(move |stream, state| {
                        let WorkerState::Input(state) = state else {
                            return;
                        };

                        println!("INPUT CALLBACK RUNNING {:?}", stream.state());

                        let Some(mut buffer) =
                            stream.dequeue_buffer()
                        else {
                            return;
                        };

                        println!("Buffer requested: {:?}", buffer.requested());

                        let datas = buffer.datas_mut();

                        if datas.is_empty() {
                            return;
                        }

                        let data = &datas[0];


                        let size = {
                            let chunk = data.chunk();

                            println!(
                                "offset={} size={} stride={}",
                                chunk.offset(),
                                chunk.size(),
                                chunk.stride(),
                            );

                            chunk.size() as usize
                        };

                        let chunk = data.chunk();

                        let offset = chunk.offset() as usize;
                        let size = chunk.size() as usize;

                        let data = &mut datas[0];

                        let Some(bytes) = data.data() else {
                            return;
                        };

                        let bytes = &bytes[offset .. offset + size];

                        let non_zero = bytes.iter().any(|b| *b != 0);

                        println!(
                            "bytes={} non_zero={}",
                            bytes.len(),
                            non_zero
                        );

                        let samples: &[f32] = unsafe {
                            std::slice::from_raw_parts(
                                bytes.as_ptr() as *const f32,
                                size / size_of::<f32>(),
                            )
                        };

                        let max = samples
                            .iter()
                            .fold(0.0f32, |m, x| m.max(x.abs()));

                        println!(
                            "samples={} max={}",
                            samples.len(),
                            max
                        );

                        state.writer.write(samples);
                    })
                    .register()?
            }
        };

        // bytes должны жить, пока существует Pod.
        let pod_bytes =
            Self::create_audio_params(&config.format)?;

        let mut params = [
            Pod::from_bytes(&pod_bytes).unwrap()
        ];

        stream.connect(
            direction,
            config.node_id,
            StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
            &mut params,
        )?;

        Ok(Self {
            _main_loop: main_loop,
            _context: context,
            _core: core,
            _stream: stream,
            _stream_listener: stream_listener,

            pending_drain,
            command_rx,

            _format: config.format,
            _node_name: config.node_name,

            _pod_bytes: pod_bytes,
        })
    }

    fn process_commands(&self) -> Result<()> {
        while let Ok(cmd) = self.command_rx.try_recv() {
            match cmd {
                WorkerCommand::Drain(done) => {
                    if self.pending_drain.borrow().is_some() {
                        // Игнорируем повторный вызов
                        continue
                    }
                    *self.pending_drain.borrow_mut() = Some(done);
                    self._stream.flush(true)?;
                }
            }
        }

        Ok(())
    }
}

impl Drop for PipeWireWorker {
    fn drop(&mut self) {
        let _ = self.drain();

        // дать PipeWire еще немного пожить
        std::thread::sleep(Duration::from_millis(200));

        self.shutdown.store(true, Ordering::Release);

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
