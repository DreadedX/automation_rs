mod ikea_outlet;
pub use self::ikea_outlet::IkeaOutlet;

mod wake_on_lan;
pub use self::wake_on_lan::WakeOnLAN;

use std::collections::HashMap;

use google_home::{GoogleHomeDevice, traits::OnOff};

use crate::mqtt::Listener;

impl_cast::impl_cast!(Device, Listener);
impl_cast::impl_cast!(Device, GoogleHomeDevice);
impl_cast::impl_cast!(Device, OnOff);

pub trait Device: AsGoogleHomeDevice + AsListener + AsOnOff {
    fn get_id(&self) -> String;
}

// @TODO Add an inner type that we can wrap with Arc<RwLock<>> to make this type a little bit nicer
// to work with
pub struct Devices {
    devices: HashMap<String, DeviceBox>,
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

pub type DeviceBox = Box<dyn Device + Sync + Send>;

impl Devices {
    pub fn new() -> Self {
        Self { devices: HashMap::new() }
    }

    pub fn add_device(&mut self, device: DeviceBox) {
        self.devices.insert(device.get_id(), device);
    }

    get_cast!(Listener);
    get_cast!(GoogleHomeDevice);
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
