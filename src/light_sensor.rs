use async_trait::async_trait;
use rumqttc::{matches, AsyncClient};
use tokio::sync::watch;
use tracing::{error, trace, debug};

use crate::{config::{MqttDeviceConfig, LightSensorConfig}, mqtt::{self, OnMqtt, BrightnessMessage}};

#[async_trait]
pub trait OnDarkness {
    async fn on_darkness(&mut self, dark: bool);
}

pub type Receiver = watch::Receiver<bool>;
type Sender = watch::Sender<bool>;

struct LightSensor {
    is_dark: Receiver,
    mqtt: MqttDeviceConfig,
    min: isize,
    max: isize,
    tx: Sender,
}

pub async fn start(mut mqtt_rx: mqtt::Receiver, config: LightSensorConfig, client: AsyncClient) -> Receiver {
    client.subscribe(config.mqtt.topic.clone(), rumqttc::QoS::AtLeastOnce).await.unwrap();

    let (tx, is_dark) = watch::channel(false);
    let mut light_sensor = LightSensor { is_dark: is_dark.clone(), mqtt: config.mqtt, min: config.min, max: config.max, tx };

    tokio::spawn(async move {
        loop {
            // @TODO Handle errors, warn if lagging
            if let Ok(message) = mqtt_rx.recv().await {
                light_sensor.on_mqtt(&message).await;
            }
        }
    });

    return is_dark;
}

#[async_trait]
impl OnMqtt for LightSensor {
    async fn on_mqtt(&mut self, message: &rumqttc::Publish) {
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
            trace!("In between min ({}) and max ({}) value, keeping current state: {}", self.min, self.max, *self.is_dark.borrow());
            *self.is_dark.borrow()
        };

        if is_dark != *self.is_dark.borrow() {
            debug!("Dark state has changed: {is_dark}");
            self.tx.send(is_dark).ok();
        }
    }
}
