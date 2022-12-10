mod ikea_outlet;

use crate::{mqtt::Listener, state::StateOnOff};

pub use self::ikea_outlet::IkeaOutlet;

pub trait Device {
    fn get_identifier(&self) -> &str;

    fn as_state_on_off(&mut self) -> Option<&mut dyn StateOnOff>;

    fn as_listener(&mut self) -> Option<&mut dyn Listener>;
}

pub struct Devices {
    devices: Vec<Box<dyn Device>>,
}

impl Devices {
    pub fn new() -> Self {
        Self { devices: Vec::new() }
    }

    pub fn add_device<T: Device + 'static>(&mut self, device: T) {
        self.devices.push(Box::new(device));
    }

    pub fn as_listeners(&mut self) -> Vec<&mut dyn Listener> {
        self.devices.iter_mut().filter_map(|device| device.as_listener()).collect()
    }

    pub fn get_device(&mut self, index: usize) -> Option<&mut dyn Device> {
        if let Some(device) = self.devices.get_mut(index) {
            return Some(device.as_mut());
        }
        return None;
    }
}

impl Listener for Devices {
    fn notify(&mut self, message: &rumqttc::Publish) {
        self.as_listeners().iter_mut().for_each(|listener| {
            listener.notify(message);
        })
    }
}
