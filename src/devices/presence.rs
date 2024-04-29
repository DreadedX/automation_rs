use std::collections::HashMap;

use async_trait::async_trait;
use automation_macro::{LuaDevice, LuaDeviceConfig};
use rumqttc::Publish;
use tracing::{debug, trace, warn};

use super::LuaDeviceCreate;
use crate::config::MqttDeviceConfig;
use crate::devices::Device;
use crate::event::{self, Event, EventChannel, OnMqtt};
use crate::messages::PresenceMessage;
use crate::mqtt::WrappedAsyncClient;

#[derive(Debug, LuaDeviceConfig)]
pub struct PresenceConfig {
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    #[device_config(from_lua, rename("event_channel"), with(|ec: EventChannel| ec.get_tx()))]
    tx: event::Sender,
    #[device_config(from_lua)]
    client: WrappedAsyncClient,
}

pub const DEFAULT_PRESENCE: bool = false;

#[derive(Debug, LuaDevice)]
pub struct Presence {
    config: PresenceConfig,
    devices: HashMap<String, bool>,
    current_overall_presence: bool,
}

#[async_trait]
impl LuaDeviceCreate for Presence {
    type Config = PresenceConfig;
    type Error = rumqttc::ClientError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = "ntfy", "Setting up Presence");

        config
            .client
            .subscribe(&config.mqtt.topic, rumqttc::QoS::AtLeastOnce)
            .await?;

        Ok(Self {
            config,
            devices: HashMap::new(),
            current_overall_presence: DEFAULT_PRESENCE,
        })
    }
}

impl Device for Presence {
    fn get_id(&self) -> String {
        "presence".to_string()
    }
}

#[async_trait]
impl OnMqtt for Presence {
    async fn on_mqtt(&mut self, message: Publish) {
        if !rumqttc::matches(&message.topic, &self.config.mqtt.topic) {
            return;
        }

        let offset = self
            .config
            .mqtt
            .topic
            .find('+')
            .or(self.config.mqtt.topic.find('#'))
            .expect("Presence::create fails if it does not contain wildcards");
        let device_name = message.topic[offset..].into();

        if message.payload.is_empty() {
            // Remove the device from the map
            debug!("State of device [{device_name}] has been removed");
            self.devices.remove(&device_name);
        } else {
            let present = match PresenceMessage::try_from(message) {
                Ok(state) => state.presence(),
                Err(err) => {
                    warn!("Failed to parse message: {err}");
                    return;
                }
            };

            debug!("State of device [{device_name}] has changed: {}", present);
            self.devices.insert(device_name, present);
        }

        let overall_presence = self.devices.iter().any(|(_, v)| *v);
        if overall_presence != self.current_overall_presence {
            debug!("Overall presence updated: {overall_presence}");
            self.current_overall_presence = overall_presence;

            if self
                .config
                .tx
                .send(Event::Presence(overall_presence))
                .await
                .is_err()
            {
                warn!("There are no receivers on the event channel");
            }
        }
    }
}
