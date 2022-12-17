mod ikea_outlet;
pub use self::ikea_outlet::IkeaOutlet;

mod test_outlet;
pub use self::test_outlet::TestOutlet;

use std::collections::HashMap;

use google_home::{Fullfillment, traits::OnOff};

use crate::mqtt::Listener;

impl_cast::impl_cast!(Device, Listener);
impl_cast::impl_cast!(Device, Fullfillment);
impl_cast::impl_cast!(Device, OnOff);

pub trait Device: Sync + Send + AsFullfillment + AsListener + AsOnOff {
    fn get_id(&self) -> String;
}

pub struct Devices {
    devices: HashMap<String, Box<dyn Device>>,
}

macro_rules! get_cast {
    ($trait:ident) => {
        paste::paste! {
            pub fn [< as_ $trait:snake s >](&mut self) -> HashMap<String, &mut dyn $trait> {
                self.devices
                    .iter_mut()
                    .filter_map(|(id, device)| {
                        if let Some(listener) = [< As $trait >]::cast_mut(device.as_mut()) {
                            return Some((id.clone(), listener));
                        };
                        return None;
                    }).collect()
            }
        }
    };
}

impl Devices {
    pub fn new() -> Self {
        Self { devices: HashMap::new() }
    }

    pub fn add_device<T: Device + 'static>(&mut self, device: T) {
        self.devices.insert(device.get_id(), Box::new(device));
    }

    get_cast!(Listener);
    get_cast!(Fullfillment);
    get_cast!(OnOff);

    pub fn get_device(&mut self, name: &str) -> Option<&mut dyn Device> {
        if let Some(device) = self.devices.get_mut(name) {
            return Some(device.as_mut());
        }
        return None;
    }
}

impl Listener for Devices {
    fn notify(&mut self, message: &rumqttc::Publish) {
        self.as_listeners().iter_mut().for_each(|(_, listener)| {
            listener.notify(message);
        })
    }
}
