use async_trait::async_trait;
use rumqttc::Publish;
use serde::Deserialize;
use tracing::{debug, trace, warn};

use crate::{
    config::MqttDeviceConfig,
    devices::Device,
    event::{self, Event, EventChannel},
    mqtt::{BrightnessMessage, OnMqtt},
};

#[async_trait]
pub trait OnDarkness: Sync + Send + 'static {
    async fn on_darkness(&mut self, dark: bool);
}

#[derive(Debug, Clone, Deserialize)]
pub struct LightSensorConfig {
    #[serde(flatten)]
    pub mqtt: MqttDeviceConfig,
    pub min: isize,
    pub max: isize,
}

pub const DEFAULT: bool = false;

#[derive(Debug)]
pub struct LightSensor {
    tx: event::Sender,
    mqtt: MqttDeviceConfig,
    min: isize,
    max: isize,
    is_dark: bool,
}

impl LightSensor {
    pub fn new(config: LightSensorConfig, event_channel: &EventChannel) -> Self {
        Self {
            tx: event_channel.get_tx(),
            mqtt: config.mqtt,
            min: config.min,
            max: config.max,
            is_dark: DEFAULT,
        }
    }
}

impl Device for LightSensor {
    fn get_id(&self) -> &str {
        "light_sensor"
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
