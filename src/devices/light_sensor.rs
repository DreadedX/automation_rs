use async_trait::async_trait;
use automation_macro::{LuaDevice, LuaDeviceConfig};
use rumqttc::Publish;
use tracing::{debug, trace, warn};

use crate::config::MqttDeviceConfig;
use crate::device_manager::DeviceConfig;
use crate::devices::Device;
use crate::error::DeviceConfigError;
use crate::event::{self, Event, OnMqtt};
use crate::helper::TxHelper;
use crate::messages::BrightnessMessage;

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct LightSensorConfig {
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    pub min: isize,
    pub max: isize,
    #[device_config(rename = "event_channel", user_data, with = "TxHelper")]
    pub tx: event::Sender,
}

pub const DEFAULT: bool = false;

// TODO: The light sensor should get a list of devices that it should inform

#[async_trait]
impl DeviceConfig for LightSensorConfig {
    async fn create(&self, identifier: &str) -> Result<Box<dyn Device>, DeviceConfigError> {
        let device = LightSensor {
            identifier: identifier.into(),
            // Add helper type that does this conversion for us
            config: self.clone(),
            is_dark: DEFAULT,
        };

        Ok(Box::new(device))
    }
}

#[derive(Debug, LuaDevice)]
pub struct LightSensor {
    identifier: String,
    #[config]
    config: LightSensorConfig,

    is_dark: bool,
}

impl Device for LightSensor {
    fn get_id(&self) -> &str {
        &self.identifier
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
