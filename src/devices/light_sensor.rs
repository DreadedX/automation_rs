use async_trait::async_trait;
use automation_macro::LuaDeviceConfig;
use rumqttc::Publish;
use tracing::{debug, trace, warn};

use super::LuaDeviceCreate;
use crate::config::MqttDeviceConfig;
use crate::devices::Device;
use crate::event::{self, Event, EventChannel, OnMqtt};
use crate::messages::BrightnessMessage;
use crate::mqtt::WrappedAsyncClient;

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct LightSensorConfig {
    pub identifier: String,
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    pub min: isize,
    pub max: isize,
    #[device_config(rename("event_channel"), from_lua, with(|ec: EventChannel| ec.get_tx()))]
    pub tx: event::Sender,
    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

const DEFAULT: bool = false;

#[derive(Debug)]
pub struct LightSensor {
    config: LightSensorConfig,

    is_dark: bool,
}

#[async_trait]
impl LuaDeviceCreate for LightSensor {
    type Config = LightSensorConfig;
    type Error = rumqttc::ClientError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.identifier, "Setting up LightSensor");

        config
            .client
            .subscribe(&config.mqtt.topic, rumqttc::QoS::AtLeastOnce)
            .await?;

        Ok(Self {
            config,
            is_dark: DEFAULT,
        })
    }
}

impl Device for LightSensor {
    fn get_id(&self) -> String {
        self.config.identifier.clone()
    }
}

#[async_trait]
impl OnMqtt for LightSensor {
    async fn on_mqtt(&mut self, message: Publish) {
        if !rumqttc::matches(&message.topic, &self.config.mqtt.topic) {
            return;
        }

        let illuminance = match BrightnessMessage::try_from(message) {
            Ok(state) => state.illuminance(),
            Err(err) => {
                warn!("Failed to parse message: {err}");
                return;
            }
        };

        debug!("Illuminance: {illuminance}");
        let is_dark = if illuminance <= self.config.min {
            trace!("It is dark");
            true
        } else if illuminance >= self.config.max {
            trace!("It is light");
            false
        } else {
            trace!(
                "In between min ({}) and max ({}) value, keeping current state: {}",
                self.config.min,
                self.config.max,
                self.is_dark
            );
            self.is_dark
        };

        if is_dark != self.is_dark {
            debug!("Dark state has changed: {is_dark}");
            self.is_dark = is_dark;

            if self.config.tx.send(Event::Darkness(is_dark)).await.is_err() {
                warn!("There are no receivers on the event channel");
            }
        }
    }
}
