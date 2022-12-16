use google_home::errors::ErrorCode;
use google_home::{GoogleHomeDevice, device, types::Type, traits};
use rumqttc::{Client, Publish};
use serde::{Deserialize, Serialize};

use crate::devices::Device;
use crate::mqtt::Listener;
use crate::zigbee::Zigbee;

pub struct IkeaOutlet {
    name: String,
    zigbee: Zigbee,
    client: Client,
    last_known_state: bool,
}

impl IkeaOutlet {
    pub fn new(name: String, zigbee: Zigbee, mut client: Client) -> Self {
        client.subscribe(zigbee.get_topic(), rumqttc::QoS::AtLeastOnce).unwrap();
        Self{ name, zigbee, client, last_known_state: false }
    }
}

impl Device for IkeaOutlet {
    fn get_id(&self) -> String {
        self.zigbee.get_friendly_name().into()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct StateMessage {
    state: String
}

impl From<&Publish> for StateMessage {
    fn from(p: &Publish) -> Self {
        let parsed = match serde_json::from_slice(&p.payload) {
            Ok(outlet) => outlet,
            Err(err) => {
                panic!("{}", err);
            }
        };

        parsed
    }
}

impl Listener for IkeaOutlet {
    fn notify(&mut self, message: &Publish) {
        // Update the internal state based on what the device has reported
        if message.topic == self.zigbee.get_topic() {
            let state = StateMessage::from(message);

            print!("Updating state: {} => ", self.last_known_state);
            self.last_known_state = state.state == "ON";
            println!("{}", self.last_known_state);
        }
    }
}

impl GoogleHomeDevice for IkeaOutlet {
    fn get_device_type(&self) -> Type {
        Type::Outlet
    }

    fn get_device_name(&self) -> device::Name {
        device::Name::new(&self.name)
    }

    fn get_id(&self) -> String {
        Device::get_id(self)
    }

    fn is_online(&self) -> bool {
        true
    }
}

impl traits::OnOff for IkeaOutlet {
    fn is_on(&self) -> Result<bool, ErrorCode> {
        Ok(self.last_known_state)
    }

    fn set_on(&mut self, on: bool) -> Result<(), ErrorCode> {
        let topic = self.zigbee.get_topic().to_owned();
        let message = StateMessage{
            state: if on {
                "ON".to_owned()
            } else {
                "OFF".to_owned()
            }
        };

        // @TODO Handle potential error here
        self.client.publish(topic + "/set", rumqttc::QoS::AtLeastOnce, false, serde_json::to_string(&message).unwrap()).unwrap();

        Ok(())
    }
}
