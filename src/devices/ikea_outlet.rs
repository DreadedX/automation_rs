use std::time::Duration;

use google_home::errors::ErrorCode;
use google_home::{GoogleHomeDevice, device, types::Type, traits};
use rumqttc::{AsyncClient, Publish};
use serde::{Deserialize, Serialize};
use log::{debug, trace, warn};
use tokio::task::JoinHandle;

use crate::config::{KettleConfig, InfoConfig, MqttDeviceConfig};
use crate::devices::Device;
use crate::mqtt::Listener;
use crate::presence::OnPresence;

pub struct IkeaOutlet {
    identifier: String,
    info: InfoConfig,
    mqtt: MqttDeviceConfig,
    kettle: Option<KettleConfig>,

    client: AsyncClient,
    last_known_state: bool,
    handle: Option<JoinHandle<()>>,
}

impl IkeaOutlet {
    pub fn new(identifier: String, info: InfoConfig, mqtt: MqttDeviceConfig, kettle: Option<KettleConfig>, client: AsyncClient) -> Self {
        let c = client.clone();
        let t = mqtt.topic.clone();
        // @TODO Handle potential errors here
        tokio::spawn(async move {
            c.subscribe(t, rumqttc::QoS::AtLeastOnce).await.unwrap();
        });

        Self{ identifier, info, mqtt, kettle, client, last_known_state: false, handle: None }
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

impl TryFrom<&Publish> for StateMessage {
    type Error = anyhow::Error;

    fn try_from(message: &Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(anyhow::anyhow!("Invalid message payload received: {:?}", message.payload)))
    }
}

impl Listener for IkeaOutlet {
    fn notify(&mut self, message: &Publish) {
        // Update the internal state based on what the device has reported
        if message.topic != self.mqtt.topic {
            return;
        }

        let new_state = match StateMessage::try_from(message) {
            Ok(state) => state,
            Err(err) => {
                warn!("Failed to parse message: {err}");
                return;
            }
        }.state == "ON";

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
            let kettle = match &self.kettle {
                Some(kettle) => kettle,
                None => return,
            };

            let timeout = match kettle.timeout.clone() {
                Some(timeout) => timeout,
                None => {
                    trace!("Outlet is a kettle without timeout");
                    return;
                },
            };

            // Turn the kettle of after the specified timeout
            // @TODO Impl Drop for IkeaOutlet that will abort the handle if the IkeaOutlet
            // get dropped
            let client = self.client.clone();
            let topic = self.mqtt.topic.clone();
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
        }
    }
}

impl OnPresence for IkeaOutlet {
    fn on_presence(&mut self, presence: bool) {
        // Turn off the outlet when we leave the house
        if !presence {
            let client = self.client.clone();
            let topic = self.mqtt.topic.clone();
            tokio::spawn(async move {
            set_on(client, topic, false).await;
            });
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

    fn will_report_state(&self) -> bool {
        // @TODO Implement state reporting
        false
    }
}

impl traits::OnOff for IkeaOutlet {
    fn is_on(&self) -> Result<bool, ErrorCode> {
        Ok(self.last_known_state)
    }

    fn set_on(&mut self, on: bool) -> Result<(), ErrorCode> {
        let client = self.client.clone();
        let topic = self.mqtt.topic.clone();
        tokio::spawn(async move {
            set_on(client, topic, on).await;
        });

        Ok(())
    }
}
