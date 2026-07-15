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
use pipewire::stream::{StreamFlags, StreamListener, StreamRc};

use crate::audio::{AudioFormat, FrameConsumer, FrameId};
use crate::core::error::Result;

pub struct PipeWireWorker {
    thread: Option<JoinHandle<()>>,
}

struct OutputState {
    consumer: FrameConsumer,

    current_frame: Option<FrameId>,
    current_offset: usize,
}

struct PipeWireSession {
    main_loop: MainLoopRc,
    context: ContextRc,
    core: CoreRc,

    stream: StreamRc,
    listener: StreamListener<OutputState>,

    format: AudioFormat,
    node_name: String,
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

        let mut ctx = PipeWireSession::new(config)?;

        while !shutdown.load(Ordering::Acquire) {
            ctx.main_loop
                .loop_()
                .iterate(Timeout::Finite(Duration::from_millis(10)));
        }

        Ok(())
    }
}

impl PipeWireSession {
    pub fn new(
        config: WorkerConfig,
    ) -> Result<Self> {
        pipewire::init();

        let main_loop = MainLoopRc::new(None)?;
        let context = ContextRc::new(&main_loop, None)?;
        let core = context.connect_rc(None)?;

        let props = properties!(
            *keys::MEDIA_TYPE => "Audio",
            *keys::MEDIA_CATEGORY => "Playback",
            *keys::MEDIA_ROLE => "Communication",
            *keys::NODE_NAME => config.node_name.clone(),
            *keys::NODE_DESCRIPTION => config.node_name.clone(),
        );

        let stream = StreamRc::new(
            core.clone(),
            &config.node_name,
            props.into(),
        )?;

        let listener = stream
            .add_local_listener_with_user_data(OutputState {
                consumer: config.consumer,
                current_frame: None,
                current_offset: 0,
            })
            .state_changed(|stream, _, old, new| {
                println!(
                    "{}: {:?} -> {:?}",
                    stream.name(),
                    old,
                    new
                );
            })
            .register()?;

        stream.connect(
            spa::utils::Direction::Output,
            None,
            StreamFlags::AUTOCONNECT
                | StreamFlags::MAP_BUFFERS,
            &mut [],
        )?;

        Ok(Self {
            main_loop,
            context,
            core,
            stream,
            listener,
            format: config.format,
            node_name: config.node_name,
        })
    }
}
