use std::cell::RefCell;
use std::collections::HashMap;
use std::ptr::NonNull;
use std::rc::Rc;
use std::time::Duration;
use pipewire as pw;
use pipewire::loop_::Timeout;
use pipewire::proxy::{Listener, ProxyListener, ProxyT};
use pipewire::registry::GlobalObject;
use pipewire::spa::param::audio::AudioInfoRaw;
use pipewire::spa::param::format::FormatProperties;
use pipewire::spa::pod::deserialize::PodDeserializer;
use pipewire::spa::pod::{ChoiceValue, Pod, Value};
use pipewire::spa::utils::Choice;
use pipewire::spa::utils::dict::DictRef;
use pipewire::types::ObjectType;
use symphonia::core::audio::conv::IntoSample;
use crate::core::error::Result;
use crate::audio::{AudioDeviceFactory, AudioDeviceId, AudioDeviceInfo, AudioDevices, AudioDirection, AudioFormat, Endianness, HardwareDeviceConfig, PcmFormat, VirtualDeviceConfig};

#[derive(Debug, Clone)]
struct NodeInfo {
    id: u32,
    properties: HashMap<String, String>,
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

    fn add(
        &mut self,
        proxy: Box<dyn ProxyT>,
        listener: Box<dyn Listener>,
    ) {
        let id = proxy.upcast_ref().id();

        self.proxies.insert(id, proxy);

        self.listeners
            .entry(id)
            .or_default()
            .push(listener);
    }

    fn add_proxy_listener(
        &mut self,
        proxy_id: u32,
        listener: ProxyListener,
    ) {
        self.listeners
            .entry(proxy_id)
            .or_default()
            .push(Box::new(listener));
    }

    fn remove(
        &mut self,
        proxy_id: u32,
    ) {
        self.proxies.remove(&proxy_id);
        self.listeners.remove(&proxy_id);
    }
}

pub struct PipeWireDeviceFactory;

impl AudioDeviceFactory for PipeWireDeviceFactory {
    fn enumerate_devices(
        &self,
    ) -> Result<Vec<AudioDeviceInfo>> {
        let nodes = enumerate_nodes()?;

        let mut devices = Vec::new();

        for node in nodes {
            let Some(media_class) =
                node.properties.get("media.class")
            else {
                continue;
            };

            let direction = match media_class.as_str() {
                "Audio/Source" => AudioDirection::Input,
                "Audio/Sink" => AudioDirection::Output,
                _ => continue,
            };

            let name = node
                .properties
                .get("node.description")
                .or_else(|| node.properties.get("node.nick"))
                .or_else(|| node.properties.get("node.name"))
                .cloned()
                .unwrap_or_else(|| format!("Node {}", node.id));

            devices.push(AudioDeviceInfo {
                id: AudioDeviceId::Numeric(node.id as u64),
                name,
                direction,
                formats: Vec::new(),
                default_format: AudioFormat {
                    sample_rate: 48_000,
                    channels: 2,
                    sample_format: PcmFormat::F32(Endianness::Little),
                },
                default: false,
                virtual_device: false,
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

fn enumerate_nodes() -> Result<Vec<NodeInfo>> {
    let main_loop = pw::main_loop::MainLoopRc::new(None)?;
    let context = pw::context::ContextRc::new(&main_loop, None)?;
    let core = context.connect_rc(None)?;
    let registry = core.get_registry_rc()?;
    let registry_weak = registry.downgrade();

    let nodes = Rc::new(
        RefCell::new(Vec::<NodeInfo>::new())
    );

    let bound = Rc::new(
        RefCell::new(BoundObjects::new())
    );
    let bound_ref = bound.clone();

    let nodes_ref = nodes.clone();

    let _listener = registry
        .add_listener_local()
        .global(move |obj: &GlobalObject<&DictRef>| {
            let Some(registry) = registry_weak.upgrade() else {
                return;
            };

            /*
            if obj.type_ != ObjectType::Node {
                return;
            }
             */

            println!("-------------------------------");
            println!("Node {} - {:?}", obj.id, obj.type_);

            if obj.type_ == ObjectType::Node {
                let node: pw::node::Node = match registry.bind(obj) {
                    Ok(node) => node,
                    Err(err) => {
                        eprintln!("bind failed: {err}");
                        return;
                    }
                };

                let node_listener = node
                    .add_listener_local()
                    .info(|info| {

                        println!("====================");
                        println!("NODE INFO");

                        for param in info.params() {
                            println!("{param:#?}");
                        }

                    })
                    .param(|seq, id, index, next, pod| {
                        println!("--------------------");
                        println!("seq   = {seq}");
                        println!("id    = {:?}", id);
                        println!("index = {index}");
                        println!("next  = {next}");

                        let Some(pod) = pod else {
                            return;
                        };

                        if let Ok((_, Value::Object(obj))) = PodDeserializer::deserialize_from(pod.as_bytes()) {
                            // 2. Итерируемся по свойствам объекта (они уже полностью распарсены компилятором)
                            for prop in obj.properties {
                                let key = FormatProperties::from_raw(prop.key);
                                match key {
                                    FormatProperties::AudioFormat => {
                                        println!("AudioFormat")
                                    }

                                    FormatProperties::AudioRate => {
                                        println!("AudioRate")
                                    }

                                    FormatProperties::AudioChannels => {
                                        println!("AudioCannels")
                                    }

                                    FormatProperties::AudioPosition => {
                                        println!("AudioPosition")
                                    }

                                    _ => {
                                        println!("Other")
                                    }
                                }

                                match &prop.value {
                                    Value::Choice(choice_value) => {
                                        // Здесь вы получаете готовый ChoiceValue со всеми вариантами
                                        println!("Choice: ID свойства: {}, Варианты Choice: {:?}", prop.key, choice_value);
                                    }
                                    _ => {
                                        // Другие типы свойств
                                        println!("ID свойства: {}, Значение: {:?}", prop.key, prop.value);
                                    }
                                }
                            }
                        }
                    })
                    .register();

                node.enum_params(
                    1,
                    Some(pw::spa::param::ParamType::EnumFormat),
                    0,
                    u32::MAX,
                );

                bound_ref.borrow_mut().add(
                    Box::new(node),
                    Box::new(node_listener),
                );
            }

            if let Some(props) = &obj.props {
                for (k, v) in props.iter() {
                    println!("{k} = {v}");
                }
            }

            let mut props = HashMap::new();

            if let Some(p) = &obj.props {
                for (k, v) in p.iter() {
                    props.insert(
                        k.to_string(),
                        v.to_string(),
                    );
                }
            }

            nodes_ref.borrow_mut().push(NodeInfo {
                id: obj.id,
                properties: props,
            });
        })
        .register();

    core.sync(0)?;

    for _ in 0..10 {
        main_loop.loop_().iterate(Timeout::Finite(Duration::from_millis(50)));
    }

    Ok(nodes.borrow().clone())
}
