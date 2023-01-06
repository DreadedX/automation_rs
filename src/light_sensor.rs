use std::sync::Weak;

use parking_lot::RwLock;
use pollster::FutureExt as _;
use rumqttc::{AsyncClient, matches};
use tracing::{span, Level, log::{error, trace}, debug};

use crate::{config::{MqttDeviceConfig, LightSensorConfig}, mqtt::{OnMqtt, BrightnessMessage}};


pub trait OnDarkness {
    fn on_darkness(&mut self, dark: bool);
}

pub struct LightSensor {
    listeners: Vec<Weak<RwLock<dyn OnDarkness + Sync + Send>>>,
    is_dark: bool,
    mqtt: MqttDeviceConfig,
    min: isize,
    max: isize,
}

impl LightSensor {
    pub fn new(config: LightSensorConfig, client: AsyncClient) -> Self {
        client.subscribe(config.mqtt.topic.clone(), rumqttc::QoS::AtLeastOnce).block_on().unwrap();

        Self { listeners: Vec::new(), is_dark: false, mqtt: config.mqtt, min: config.min, max: config.max }
    }

    pub fn add_listener<T: OnDarkness + Sync + Send + 'static>(&mut self, listener: Weak<RwLock<T>>) {
        self.listeners.push(listener);
    }

    pub fn notify(dark: bool, listeners: Vec<Weak<RwLock<dyn OnDarkness + Sync + Send>>>) {
        let _span = span!(Level::TRACE, "darkness_update").entered();
        listeners.into_iter().for_each(|listener| {
            if let Some(listener) = listener.upgrade() {
                listener.write().on_darkness(dark);
            }
        })
    }
}

impl OnMqtt for LightSensor {
    fn on_mqtt(&mut self, message: &rumqttc::Publish) {
        if !matches(&message.topic, &self.mqtt.topic) {
            return;
        }

        let illuminance = match BrightnessMessage::try_from(message) {
            Ok(state) => state.illuminance(),
            Err(err) => {
                error!("Failed to parse message: {err}");
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
            trace!("In between min ({}) and max ({}) value, keeping current state: {}", self.min, self.max, self.is_dark);
            self.is_dark
        };

        if is_dark != self.is_dark {
            debug!("Dark state has changed: {is_dark}");
            self.is_dark = is_dark;
            self.listeners.retain(|listener| listener.strong_count() > 0);
            let listeners = self.listeners.clone();

            tokio::task::spawn_blocking(move || {
                LightSensor::notify(is_dark, listeners)
            });
        }
    }
}
