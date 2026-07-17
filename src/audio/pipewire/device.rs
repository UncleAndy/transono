use libspa_sys::*;
use pipewire as pw;
use pipewire::loop_::Timeout;
use pipewire::proxy::{Listener, ProxyListener, ProxyT};
use pipewire::registry::GlobalObject;
use pipewire::spa::param::format::FormatProperties;
use pipewire::spa::pod::deserialize::PodDeserializer;
use pipewire::spa::pod::{ChoiceValue, Value};
use pipewire::spa::utils::ChoiceEnum;
use pipewire::spa::utils::dict::DictRef;
use pipewire::types::ObjectType;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;

use crate::audio::{
    AudioDeviceFactory, AudioDeviceId, AudioDeviceInfo, AudioDevices, AudioDirection, AudioFormat,
    Endianness, HardwareDeviceConfig, PcmFormat, VirtualDeviceConfig,
};
use crate::core::error::Result;

const DEFAULT_AUDIO_FORMAT: AudioFormat = AudioFormat {
    sample_rate: 48_000,
    channels: 2,
    sample_format: PcmFormat::F32(Endianness::Little),
};

#[derive(Debug, Clone)]
struct PipeWireNodeInfo {
    id: u32,
    properties: HashMap<String, String>,
    default_format: Option<AudioFormat>,
}

struct BoundObjects {
    proxies: HashMap<u32, Box<dyn ProxyT>>,
    listeners: HashMap<u32, Vec<Box<dyn Listener>>>,
}

impl BoundObjects {
    fn new() -> Self {
        Self {
            proxies: HashMap::new(),
            listeners: HashMap::new(),
        }
    }

    fn add(&mut self, proxy: Box<dyn ProxyT>, listener: Box<dyn Listener>) {
        let id = proxy.upcast_ref().id();

        self.proxies.insert(id, proxy);

        self.listeners.entry(id).or_default().push(listener);
    }

    fn _add_proxy_listener(&mut self, proxy_id: u32, listener: ProxyListener) {
        self.listeners
            .entry(proxy_id)
            .or_default()
            .push(Box::new(listener));
    }

    fn _remove(&mut self, proxy_id: u32) {
        self.proxies.remove(&proxy_id);
        self.listeners.remove(&proxy_id);
    }
}

pub struct PipeWireDeviceFactory;

impl AudioDeviceFactory for PipeWireDeviceFactory {
    fn enumerate_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
        let nodes = enumerate_nodes()?;

        let mut devices = Vec::new();

        for node in nodes {
            let Some(media_class) = node.properties.get("media.class") else {
                continue;
            };

            let direction = match media_class.as_str() {
                "Audio/Source" | "Stream/Input/Audio" => AudioDirection::Input,
                "Audio/Sink" | "Stream/Output/Audio" => AudioDirection::Output,
                _ => continue,
            };

            let device_id = node.properties.get("device.id");

            let is_virtual = device_id.is_none();

            let name = node
                .properties
                .get("node.description")
                .or_else(|| node.properties.get("node.nick"))
                .or_else(|| node.properties.get("node.name"))
                .cloned()
                .unwrap_or_else(|| format!("Node {}", node.id));

            let default_format = node.default_format.unwrap_or(DEFAULT_AUDIO_FORMAT);

            devices.push(AudioDeviceInfo {
                id: AudioDeviceId::Numeric(node.id as u64),
                name,
                direction,
                formats: Vec::new(),
                default_format,
                default: false,
                virtual_device: is_virtual,
            });
        }

        Ok(devices)
    }

    fn open_hardware(&self, config: &HardwareDeviceConfig) -> Result<AudioDevices> {
        todo!()
    }

    fn create_virtual(&self, config: &VirtualDeviceConfig) -> Result<AudioDevices> {
        todo!()
    }
}

#[derive(Default, Debug)]
struct DefaultFormatBuilder {
    sample_format: Option<PcmFormat>,
    sample_rate: Option<u32>,
    channels: Option<u16>,
}

impl DefaultFormatBuilder {
    fn build(self) -> Option<AudioFormat> {
        Some(AudioFormat {
            sample_rate: self.sample_rate?,
            channels: self.channels?,
            sample_format: self.sample_format?,
        })
    }
}

fn enumerate_nodes() -> Result<Vec<PipeWireNodeInfo>> {
    let main_loop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&main_loop, None)?;
    let core = context.connect_rc(None)?;
    let registry = core.get_registry_rc()?;
    let registry_weak = registry.downgrade();

    let nodes = Rc::new(RefCell::new(HashMap::<u32, PipeWireNodeInfo>::new()));

    let bound = Rc::new(RefCell::new(BoundObjects::new()));
    let bound_ref = bound.clone();

    let nodes_ref = nodes.clone();

    let _listener = registry
        .add_listener_local()
        .global(move |obj: &GlobalObject<&DictRef>| {
            let Some(registry) = registry_weak.upgrade() else {
                return;
            };

            let mut props = HashMap::new();

            if let Some(p) = &obj.props {
                for (k, v) in p.iter() {
                    props.insert(k.to_string(), v.to_string());
                }
            }

            nodes_ref.borrow_mut().insert(
                obj.id,
                PipeWireNodeInfo {
                    id: obj.id,
                    properties: props.clone(),
                    default_format: None,
                },
            );

            if obj.type_ == ObjectType::Node {
                let node: pw::node::Node = match registry.bind(obj) {
                    Ok(node) => node,
                    Err(err) => {
                        eprintln!("bind failed: {err}");
                        return;
                    }
                };

                let node_id = obj.id;
                let nodes_ref = nodes_ref.clone();

                let node_listener = node
                    .add_listener_local()
                    .param(move |seq, id, index, next, pod| {
                        let Some(pod) = pod else {
                            return;
                        };

                        if let Ok((_, Value::Object(obj))) =
                            PodDeserializer::deserialize_from(pod.as_bytes())
                        {
                            let mut builder = DefaultFormatBuilder::default();

                            for prop in obj.properties {
                                match (FormatProperties::from_raw(prop.key), &prop.value) {
                                    (FormatProperties::AudioFormat, Value::Choice(choice)) => {
                                        if let Some(id) = choice_default_id(choice) {
                                            builder.sample_format = pcm_format_from_spa(id);
                                        }
                                    }

                                    (FormatProperties::AudioRate, Value::Choice(choice)) => {
                                        builder.sample_rate =
                                            choice_default_int(choice).map(|v| v as u32);
                                    }

                                    (FormatProperties::AudioChannels, Value::Choice(choice)) => {
                                        builder.channels =
                                            choice_default_int(choice).map(|v| v as u16);
                                    }

                                    (FormatProperties::AudioPosition, _) => {
                                        // Пока игнорируем
                                    }

                                    (_, _value) => {
                                        /*
                                        println!(
                                            "Unhandled property {:?}: {:?}",
                                            FormatProperties::from_raw(prop.key),
                                            value,
                                        );
                                         */
                                    }
                                }
                            }

                            if let Some(format) = builder.build() {
                                if let Some(node) = nodes_ref.borrow_mut().get_mut(&node_id) {
                                    node.default_format = Some(format);
                                }
                            }
                        }
                    })
                    .register();

                node.enum_params(1, Some(pw::spa::param::ParamType::EnumFormat), 0, u32::MAX);

                bound_ref
                    .borrow_mut()
                    .add(Box::new(node), Box::new(node_listener));
            }
        })
        .register();

    core.sync(0)?;

    for _ in 0..10 {
        main_loop
            .loop_()
            .iterate(Timeout::Finite(Duration::from_millis(50)));
    }

    Ok(nodes.borrow().values().cloned().collect())
}

fn choice_default_id(choice: &ChoiceValue) -> Option<u32> {
    match choice {
        ChoiceValue::Id(choice) => match choice.1 {
            ChoiceEnum::None(id) => Some(id.0),
            ChoiceEnum::Enum { default, .. } => Some(default.0),
            _ => None,
        },

        _ => None,
    }
}

fn choice_default_int(choice: &ChoiceValue) -> Option<i32> {
    match choice {
        ChoiceValue::Int(choice) => match choice.1 {
            ChoiceEnum::None(v) => Some(v),
            ChoiceEnum::Enum { default, .. } => Some(default),
            ChoiceEnum::Range { default, .. } => Some(default),
            _ => None,
        },

        _ => None,
    }
}

fn pcm_format_from_spa(id: u32) -> Option<PcmFormat> {
    match id {
        SPA_AUDIO_FORMAT_F32_LE => Some(PcmFormat::F32(Endianness::Little)),
        SPA_AUDIO_FORMAT_S32_LE => Some(PcmFormat::I32(Endianness::Little)),
        SPA_AUDIO_FORMAT_S16_LE => Some(PcmFormat::I16(Endianness::Little)),
        SPA_AUDIO_FORMAT_U16_LE => Some(PcmFormat::U16(Endianness::Little)),
        _ => None,
    }
}
