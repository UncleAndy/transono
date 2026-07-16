use std::io::Cursor;
use std::sync::Arc;
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

use crate::audio::{AudioFormat, FrameConsumer, FrameId};
use crate::core::error::Result;

pub struct PipeWireWorker {
    thread: Option<JoinHandle<()>>,
}

pub struct FrameReader {
    consumer: FrameConsumer,
    current: Option<FrameId>,
    offset: usize,
}

struct OutputState {
    reader: FrameReader
}

struct PipeWireSession {
    main_loop: MainLoopRc,
    context: ContextRc,
    core: CoreRc,

    stream: StreamRc,
    listener: StreamListener<OutputState>,

    format: AudioFormat,
    node_name: String,

    pod_bytes: Vec<u8>,
}

pub struct WorkerConfig {
    pub node_name: String,
    pub format: AudioFormat,
    pub consumer: FrameConsumer,
}

impl PipeWireWorker {
    pub fn spawn_output(
        consumer: FrameConsumer,
        format: AudioFormat,
        node_name: String,
    ) -> Result<Self> {
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = shutdown.clone();

        let thread = std::thread::spawn(move || {
            if let Err(e) = Self::run_output(
                thread_shutdown,
                consumer,
                format,
                node_name,
            ) {
                eprintln!("PipeWire worker failed: {e}");
            }
        });

        Ok(Self {
            thread: Some(thread),
        })
    }

    fn run_output(
        shutdown: Arc<AtomicBool>,
        consumer: FrameConsumer,
        format: AudioFormat,
        node_name: String,
    ) -> Result<()> {
        let config = WorkerConfig {
            consumer,
            format,
            node_name,
        };

        let ctx = PipeWireSession::new(config)?;

        while !shutdown.load(Ordering::Acquire) {
            ctx.main_loop
                .loop_()
                .iterate(Timeout::Finite(Duration::from_millis(10)));
        }

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

    pub fn new(config: WorkerConfig) -> Result<Self> {
        pipewire::init();

        let main_loop = MainLoopRc::new(None)?;
        let context = ContextRc::new(&main_loop, None)?;
        let core = context.connect_rc(None)?;

        let properties = properties! {
            *keys::MEDIA_TYPE        => "Audio",
            *keys::MEDIA_CATEGORY    => "Playback",
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

        let listener = stream
            .add_local_listener_with_user_data(OutputState {
                reader: FrameReader {
                    consumer: config.consumer,
                    current: None,
                    offset: 0,
                }
            })
            .state_changed(|stream, _, old, new| {
                println!(
                    "{}: {:?} -> {:?}",
                    stream.name(),
                    old,
                    new
                );
            })
            .process(move |stream, state| {
                let Some(mut buffer) = stream.dequeue_buffer() else {
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
                            bytes.len() / std::mem::size_of::<f32>(),
                        )
                    };

                    state.reader.consumer.fill_buffer(
                        &mut state.reader.current,
                        &mut state.reader.offset,
                        samples,
                    );

                    bytes.len()
                };

                let chunk = data.chunk_mut();

                *chunk.offset_mut() = 0;
                *chunk.stride_mut() = frame_stride;
                *chunk.size_mut() = size as u32;
            })
            .register()?;

        // bytes должны жить, пока существует Pod.
        let pod_bytes =
            Self::create_audio_params(&config.format)?;

        let mut params = [
            Pod::from_bytes(&pod_bytes).unwrap()
        ];

        stream.connect(
            spa::utils::Direction::Output,
            None,
            StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS,
            &mut params,
        )?;

        Ok(Self {
            main_loop,
            context,
            core,
            stream,
            listener,

            format: config.format,
            node_name: config.node_name,

            pod_bytes,
        })
    }
}
