use async_trait::async_trait;
use automation_macro::{LuaDevice, LuaDeviceConfig};
use rumqttc::Publish;
use tracing::{debug, trace, warn};

use crate::config::MqttDeviceConfig;
use crate::devices::Device;
use crate::error::DeviceConfigError;
use crate::event::{self, Event, EventChannel, OnMqtt};
use crate::messages::BrightnessMessage;

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct LightSensorConfig {
    identifier: String,
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    pub min: isize,
    pub max: isize,
    #[device_config(rename("event_channel"), from_lua, with(|ec: EventChannel| ec.get_tx()))]
    pub tx: event::Sender,
}

pub const DEFAULT: bool = false;

#[derive(Debug, LuaDevice)]
pub struct LightSensor {
    #[config]
    config: LightSensorConfig,

    is_dark: bool,
}

impl LightSensor {
    async fn create(config: LightSensorConfig) -> Result<Self, DeviceConfigError> {
        trace!(id = config.identifier, "Setting up LightSensor");
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
    fn topics(&self) -> Vec<&str> {
        vec![&self.config.mqtt.topic]
    }

    async fn on_mqtt(&mut self, message: Publish) {
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
