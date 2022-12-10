use rumqttc::{Client, Publish};
use serde::{Deserialize, Serialize};

use crate::devices::Device;
use crate::mqtt::Listener;
use crate::state::StateOnOff;
use crate::zigbee::Zigbee;

pub struct IkeaOutlet {
    zigbee: Zigbee,
    client: Client,
    last_known_state: bool,
}

impl IkeaOutlet {
    pub fn new(zigbee: Zigbee, mut client: Client) -> Self {
        client.subscribe(zigbee.get_topic(), rumqttc::QoS::AtLeastOnce).unwrap();
        Self{ zigbee, client, last_known_state: false }
    }
}

impl Device for IkeaOutlet {
    fn get_identifier(& self) -> &str {
        &self.zigbee.get_friendly_name()
    }

    fn as_state_on_off(&mut self) -> Option<&mut dyn StateOnOff> {
        Some(self)
    }

    fn as_listener(&mut self) -> Option<&mut dyn Listener> {
        Some(self)
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

impl StateOnOff for IkeaOutlet {
    // This will send a message over mqtt to update change the state of the device
    // It does not change the internal state, that gets updated when the device responds
    fn set_state(&mut self, state: bool) {
        let topic = self.zigbee.get_topic().to_owned();
        let message = StateMessage{
            state: if state {
                "ON".to_owned()
            } else {
                "OFF".to_owned()
            }
        };

        self.client.publish(topic + "/set", rumqttc::QoS::AtLeastOnce, false, serde_json::to_string(&message).unwrap()).unwrap();
    }

    fn get_state(&self) -> bool {
        self.last_known_state
    }
}
