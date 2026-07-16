use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Duration;
use pipewire as pw;
use pipewire::loop_::Timeout;
use pipewire::registry::GlobalObject;
use pipewire::spa::utils::dict::DictRef;
use pipewire::types::ObjectType;
use crate::core::error::Result;
use crate::audio::{AudioDeviceFactory, AudioDeviceId, AudioDeviceInfo, AudioDevices, AudioDirection, AudioFormat, Endianness, HardwareDeviceConfig, PcmFormat, VirtualDeviceConfig};

#[derive(Debug, Clone)]
struct NodeInfo {
    id: u32,
    properties: std::collections::HashMap<String, String>,
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
    let core = context.connect(None)?;
    let registry = core.get_registry()?;

    let nodes = Rc::new(RefCell::new(Vec::<NodeInfo>::new()));

    let nodes_ref = nodes.clone();

    let _listener = registry
        .add_listener_local()
        .global(move |obj: &GlobalObject<&DictRef>| {
            /*
            if obj.type_ != ObjectType::Node {
                return;
            }
             */

            println!("-------------------------------");
            println!("Node {} - {:?}", obj.id, obj.type_);

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
