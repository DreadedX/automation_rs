use std::time::Duration;

use google_home::errors::ErrorCode;
use google_home::{GoogleHomeDevice, device, types::Type, traits};
use rumqttc::{AsyncClient, Publish};
use tracing::{debug, trace, error};
use tokio::task::JoinHandle;
use pollster::FutureExt as _;

use crate::config::{KettleConfig, InfoConfig, MqttDeviceConfig};
use crate::devices::Device;
use crate::mqtt::{OnMqtt, OnOffMessage};
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
        // @TODO Handle potential errors here
        client.subscribe(mqtt.topic.clone(), rumqttc::QoS::AtLeastOnce).block_on().unwrap();

        Self{ identifier, info, mqtt, kettle, client, last_known_state: false, handle: None }
    }
}

async fn set_on(client: AsyncClient, topic: String, on: bool) {
    let message = OnOffMessage::new(on);

    // @TODO Handle potential errors here
    client.publish(topic + "/set", rumqttc::QoS::AtLeastOnce, false, serde_json::to_string(&message).unwrap()).await.unwrap();
}

impl Device for IkeaOutlet {
    fn get_id(&self) -> String {
        self.identifier.clone()
    }
}

impl OnMqtt for IkeaOutlet {
    fn on_mqtt(&mut self, message: &Publish) {
        // Update the internal state based on what the device has reported
        if message.topic != self.mqtt.topic {
            return;
        }

        let state = match OnOffMessage::try_from(message) {
            Ok(state) => state.state(),
            Err(err) => {
                error!(id = self.identifier, "Failed to parse message: {err}");
                return;
            }
        };

        // No need to do anything if the state has not changed
        if state == self.last_known_state {
            return;
        }

        // Abort any timer that is currently running
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }

        debug!(id = self.identifier, "Updating state to {state}");
        self.last_known_state = state;

        // If this is a kettle start a timeout for turning it of again
        if state {
            let kettle = match &self.kettle {
                Some(kettle) => kettle,
                None => return,
            };

            let timeout = match kettle.timeout.clone() {
                Some(timeout) => Duration::from_secs(timeout),
                None => {
                    trace!(id = self.identifier, "Outlet is a kettle without timeout");
                    return;
                },
            };

            // Turn the kettle of after the specified timeout
            // @TODO Impl Drop for IkeaOutlet that will abort the handle if the IkeaOutlet
            // get dropped
            let client = self.client.clone();
            let topic = self.mqtt.topic.clone();
            let id = self.identifier.clone();
            self.handle = Some(
                tokio::spawn(async move {
                    debug!(id, "Starting timeout ({timeout:?}) for kettle...");
                    tokio::time::sleep(timeout).await;
                    // @TODO We need to call set_on(false) in order to turn the device off
                    // again, how are we going to do this?
                    debug!(id, "Turning kettle off!");
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
            debug!(id = self.identifier, "Turning device off");
            set_on(self.client.clone(), self.mqtt.topic.clone(), false).block_on();
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
        set_on(self.client.clone(), self.mqtt.topic.clone(), on).block_on();

        Ok(())
    }
}
