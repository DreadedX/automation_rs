use google_home::errors::ErrorCode;
use google_home::{GoogleHomeDevice, device, types::Type, traits};
use rumqttc::{AsyncClient, Publish};
use serde::{Deserialize, Serialize};
use log::debug;

use crate::devices::Device;
use crate::mqtt::Listener;

pub struct IkeaOutlet {
    identifier: String,
    name: String,
    room: Option<String>,
    topic: String,

    kettle: bool,

    client: AsyncClient,
    last_known_state: bool,
}

impl IkeaOutlet {
    pub fn new(identifier: String, name: String, room: Option<String>, kettle: bool, topic: String, client: AsyncClient) -> Self {
        let c = client.clone();
        let t = topic.clone();
        // @TODO Handle potential errors here
        tokio::spawn(async move {
            c.subscribe(t, rumqttc::QoS::AtLeastOnce).await.unwrap();
        });

        Self{ identifier, name, room, kettle, topic, client, last_known_state: false }
    }
}

impl Device for IkeaOutlet {
    fn get_id(&self) -> String {
        self.identifier.clone()
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
        if message.topic == self.topic {
            let state = StateMessage::from(message);

            let new_state = state.state == "ON";
            debug!("Updating state: {} => {}", self.last_known_state, new_state);
            self.last_known_state = new_state;
        }
    }
}

impl GoogleHomeDevice for IkeaOutlet {
    fn get_device_type(&self) -> Type {
        if self.kettle {
            Type::Kettle
        } else {
            Type::Outlet
        }
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

    fn get_room_hint(&self) -> Option<String> {
        self.room.clone()
    }
}

impl traits::OnOff for IkeaOutlet {
    fn is_on(&self) -> Result<bool, ErrorCode> {
        Ok(self.last_known_state)
    }

    fn set_on(&mut self, on: bool) -> Result<(), ErrorCode> {
        let message = StateMessage{
            state: if on {
                "ON".to_owned()
            } else {
                "OFF".to_owned()
            }
        };

        // @TODO Handle potential errors here
        let client = self.client.clone();
        let topic = self.topic.to_owned();
        tokio::spawn(async move {
            client.publish(topic + "/set", rumqttc::QoS::AtLeastOnce, false, serde_json::to_string(&message).unwrap()).await.unwrap();
        });

        Ok(())
    }
}
