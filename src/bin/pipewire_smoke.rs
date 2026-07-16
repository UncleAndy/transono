use pipewire as pw;
use pw::{
    keys,
    properties::properties,
    spa::{
        self,
        pod::Pod,
    },
    stream::StreamFlags,
};
use libspa_sys as spa_sys;

pub const DEFAULT_RATE: u32 = 48000;
pub const DEFAULT_CHANNELS: u32 = 2;
pub const DEFAULT_VOLUME: f64 = 0.01;
pub const PI_2: f64 = std::f64::consts::PI + std::f64::consts::PI;
pub const CHAN_SIZE: usize = std::mem::size_of::<f32>();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pw::init();

    println!("Initializing PipeWire...");

    let mainloop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&mainloop, None)?;
    let core = context.connect_rc(None)?;

    println!("Connected to PipeWire.");

    let stream = pw::stream::StreamBox::new(
        &core,
        "pipewire-smoke",
        properties! {
            *keys::MEDIA_TYPE => "Audio",
            *keys::MEDIA_CATEGORY => "Playback",
            *keys::MEDIA_ROLE => "Communication",
            *keys::NODE_NAME => "pipewire_smoke",
            *keys::NODE_DESCRIPTION => "PipeWire Smoke Test",
        },
    )?;

    let data: f64 = 0.0;

    let _listener = stream
        .add_local_listener_with_user_data(data)
        .state_changed(|_, _, old, new| {
            println!("State: {:?} -> {:?}", old, new);
        })
        .param_changed(|_, _, id, param| {
            println!("Param changed: {}", id);

            if let Some(pod) = param {
                println!("param bytes = {}", pod.as_bytes().len());
            }
        })
        .add_buffer(|_, _, buffer| {
            println!("add_buffer {:p}", buffer);
        })
        .remove_buffer(|_, _, buffer| {
            println!("remove_buffer {:p}", buffer);
        })
        .process(|stream, acc| {
            println!("process()");
            println!("state = {:?}", stream.state());

            match stream.dequeue_buffer() {
                None => {
                    println!("  no buffer");
                }

                Some(mut buffer) => {
                    let datas = buffer.datas_mut();
                    let stride = CHAN_SIZE * DEFAULT_CHANNELS as usize;
                    let data = &mut datas[0];
                    let n_frames = if let Some(slice) = data.data() {
                        let n_frames = slice.len() / stride;
                        for i in 0..n_frames {
                            *acc += PI_2 * 440.0 / DEFAULT_RATE as f64;
                            if *acc >= PI_2 {
                                *acc -= PI_2
                            }
                            let val = (f64::sin(*acc) * DEFAULT_VOLUME * 16.0) as f32;
                            for c in 0..DEFAULT_CHANNELS {
                                let start = i * stride + (c as usize * CHAN_SIZE);
                                let end = start + CHAN_SIZE;
                                let chan = &mut slice[start..end];
                                chan.copy_from_slice(&f32::to_le_bytes(val));
                            }
                        }
                        n_frames
                    } else {
                        0
                    };
                    let chunk = data.chunk_mut();
                    *chunk.offset_mut() = 0;
                    *chunk.stride_mut() = stride as _;
                    *chunk.size_mut() = (stride * n_frames) as _;
                }
            }
        })
        .register()?;

    println!("Creating audio format...");

    let mut audio_info = spa::param::audio::AudioInfoRaw::new();
    audio_info.set_format(spa::param::audio::AudioFormat::F32LE);
    audio_info.set_rate(48_000);
    audio_info.set_channels(2);

    let mut position = [0; spa::param::audio::MAX_CHANNELS];
    position[0] = spa_sys::SPA_AUDIO_CHANNEL_FL;
    position[1] = spa_sys::SPA_AUDIO_CHANNEL_FR;
    audio_info.set_position(position);

    let values: Vec<u8> = spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &spa::pod::Value::Object(spa::pod::Object {
            type_: spa_sys::SPA_TYPE_OBJECT_Format,
            id: spa_sys::SPA_PARAM_EnumFormat,
            properties: audio_info.into(),
        }),
    )?
        .0
        .into_inner();

    let mut params = [Pod::from_bytes(&values).unwrap()];

    println!("Connecting stream...");

    stream.connect(
        spa::utils::Direction::Output,
        None,
        StreamFlags::AUTOCONNECT | StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
        &mut params,
    )?;

    println!("Running main loop...");

    mainloop.run();

    Ok(())
}
