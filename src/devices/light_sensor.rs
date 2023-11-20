use async_trait::async_trait;
use rumqttc::Publish;
use serde::Deserialize;
use tracing::{debug, trace, warn};

use crate::config::MqttDeviceConfig;
use crate::device_manager::{ConfigExternal, DeviceConfig};
use crate::devices::Device;
use crate::error::DeviceConfigError;
use crate::event::{self, Event, OnMqtt};
use crate::messages::BrightnessMessage;

#[derive(Debug, Clone, Deserialize)]
pub struct LightSensorConfig {
    #[serde(flatten)]
    pub mqtt: MqttDeviceConfig,
    pub min: isize,
    pub max: isize,
}

pub const DEFAULT: bool = false;

// TODO: The light sensor should get a list of devices that it should inform

#[async_trait]
impl DeviceConfig for LightSensorConfig {
    async fn create(
        self,
        identifier: &str,
        ext: &ConfigExternal,
    ) -> Result<Box<dyn Device>, DeviceConfigError> {
        let device = LightSensor {
            identifier: identifier.into(),
            tx: ext.event_channel.get_tx(),
            mqtt: self.mqtt,
            min: self.min,
            max: self.max,
            is_dark: DEFAULT,
        };

        Ok(Box::new(device))
    }
}

#[derive(Debug)]
pub struct LightSensor {
    identifier: String,
    tx: event::Sender,
    mqtt: MqttDeviceConfig,
    min: isize,
    max: isize,
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
        vec![&self.mqtt.topic]
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
        let is_dark = if illuminance <= self.min {
            trace!("It is dark");
            true
        } else if illuminance >= self.max {
            trace!("It is light");
            false
        } else {
            trace!(
                "In between min ({}) and max ({}) value, keeping current state: {}",
                self.min,
                self.max,
                self.is_dark
            );
            self.is_dark
        };

        if is_dark != self.is_dark {
            debug!("Dark state has changed: {is_dark}");
            self.is_dark = is_dark;

            if self.tx.send(Event::Darkness(is_dark)).await.is_err() {
                warn!("There are no receivers on the event channel");
            }
        }
    }
}
