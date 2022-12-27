use std::time::Duration;

use google_home::errors::ErrorCode;
use google_home::{GoogleHomeDevice, device, types::Type, traits};
use rumqttc::{AsyncClient, Publish};
use serde::{Deserialize, Serialize};
use log::{debug, trace};
use tokio::task::JoinHandle;

use crate::config::{KettleConfig, InfoConfig, ZigbeeDeviceConfig};
use crate::devices::Device;
use crate::mqtt::Listener;

pub struct IkeaOutlet {
    identifier: String,
    info: InfoConfig,
    zigbee: ZigbeeDeviceConfig,
    kettle: Option<KettleConfig>,

    client: AsyncClient,
    last_known_state: bool,
    handle: Option<JoinHandle<()>>,
}

impl IkeaOutlet {
    pub fn new(identifier: String, info: InfoConfig, zigbee: ZigbeeDeviceConfig, kettle: Option<KettleConfig>, client: AsyncClient) -> Self {
        let c = client.clone();
        let t = zigbee.topic.clone();
        // @TODO Handle potential errors here
        tokio::spawn(async move {
            c.subscribe(t, rumqttc::QoS::AtLeastOnce).await.unwrap();
        });

        Self{ identifier, info, zigbee, kettle, client, last_known_state: false, handle: None }
    }
}

async fn set_on(client: AsyncClient, topic: String, on: bool) {
    let message = StateMessage{
        state: if on {
            "ON".to_owned()
        } else {
            "OFF".to_owned()
        }
    };

    // @TODO Handle potential errors here
    client.publish(topic + "/set", rumqttc::QoS::AtLeastOnce, false, serde_json::to_string(&message).unwrap()).await.unwrap();
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
        if message.topic == self.zigbee.topic {
            let new_state = StateMessage::from(message).state == "ON";

            // No need to do anything if the state has not changed
            if new_state == self.last_known_state {
                return;
            }

            // Abort any timer that is currently running
            if let Some(handle) = self.handle.take() {
                handle.abort();
            }

            trace!("Updating state: {} => {}", self.last_known_state, new_state);
            self.last_known_state = new_state;

            // If this is a kettle start a timeout for turning it of again
            if new_state {
                if let Some(kettle) = &self.kettle {
                    if let Some(timeout) = kettle.timeout.clone() {
                        let client = self.client.clone();
                        let topic = self.zigbee.topic.clone();

                        // Turn the kettle of after the specified timeout
                        // @TODO Impl Drop for IkeaOutlet that will abort the handle if the IkeaOutlet
                        // get dropped
                        self.handle = Some(
                            tokio::spawn(async move {
                                debug!("Starting timeout ({timeout}s) for kettle...");
                                tokio::time::sleep(Duration::from_secs(timeout)).await;
                                // @TODO We need to call set_on(false) in order to turn the device off
                                // again, how are we going to do this?
                                debug!("Turning kettle off!");
                                set_on(client, topic, false).await;
                            })
                        );
                    } else {
                        trace!("Outlet is a kettle without timeout");
                    }

                }
            }
        }
    }
}

impl GoogleHomeDevice for IkeaOutlet {
    fn get_device_type(&self) -> Type {
        if self.kettle.is_some() {
            Type::Kettle
        } else {
            Type::Outlet
        }
    }

    fn get_device_name(&self) -> device::Name {
        device::Name::new(&self.info.name)
    }

    fn get_id(&self) -> String {
        Device::get_id(self)
    }

    fn is_online(&self) -> bool {
        true
    }

    fn get_room_hint(&self) -> Option<String> {
        self.info.room.clone()
    }
}

impl traits::OnOff for IkeaOutlet {
    fn is_on(&self) -> Result<bool, ErrorCode> {
        Ok(self.last_known_state)
    }

    fn set_on(&mut self, on: bool) -> Result<(), ErrorCode> {
        let client = self.client.clone();
        let topic = self.zigbee.topic.clone();
        tokio::spawn(async move {
            set_on(client, topic, on).await;
        });

        Ok(())
    }
}
