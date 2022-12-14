mod ikea_outlet;

use std::collections::HashMap;

use crate::{mqtt::Listener, state::StateOnOff};

pub use self::ikea_outlet::IkeaOutlet;

macro_rules! add_cast_for {
    ($i:ident) => {
        paste::paste! {
            pub trait [< As $i>] {
                fn cast(&self) -> Option<&dyn $i> {
                    None
                }
                fn cast_mut(&mut self) -> Option<&mut dyn $i> {
                    None
                }
            }
            impl<T: $i> [< As $i>] for T {
                fn cast(&self) -> Option<&dyn $i> {
                    Some(self)
                }
                fn cast_mut(&mut self) -> Option<&mut dyn $i> {
                    Some(self)
                }
            }
        }
    };
}

add_cast_for!(Listener);
add_cast_for!(StateOnOff);

pub trait Device: AsListener + AsStateOnOff {
    fn get_identifier(&self) -> &str;
}

pub struct Devices {
    devices: HashMap<String, Box<dyn Device>>,
}

impl Devices {
    pub fn new() -> Self {
        Self { devices: HashMap::new() }
    }

    pub fn add_device<T: Device + 'static>(&mut self, device: T) {
        self.devices.insert(device.get_identifier().to_owned(), Box::new(device));
    }

    pub fn get_listeners(&mut self) -> Vec<&mut dyn Listener> {
        self.devices.iter_mut().filter_map(|(_, device)| AsListener::cast_mut(device.as_mut())).collect()
    }

    pub fn get_device(&mut self, name: &str) -> Option<&mut dyn Device> {
        if let Some(device) = self.devices.get_mut(name) {
            return Some(device.as_mut());
        }
        return None;
    }
}

impl Listener for Devices {
    fn notify(&mut self, message: &rumqttc::Publish) {
        self.get_listeners().iter_mut().for_each(|listener| {
            listener.notify(message);
        })
    }
}
