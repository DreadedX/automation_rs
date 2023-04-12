use async_trait::async_trait;
use rumqttc::{matches, AsyncClient};
use tokio::sync::watch;
use tracing::{debug, error, trace};

use crate::{
    config::{LightSensorConfig, MqttDeviceConfig},
    error::LightSensorError,
    mqtt::{self, BrightnessMessage, OnMqtt},
};

#[async_trait]
pub trait OnDarkness: Sync + Send + 'static {
    async fn on_darkness(&mut self, dark: bool);
}

pub type Receiver = watch::Receiver<bool>;
type Sender = watch::Sender<bool>;

#[derive(Debug)]
struct LightSensor {
    mqtt: MqttDeviceConfig,
    min: isize,
    max: isize,
    tx: Sender,
    is_dark: Receiver,
}

impl LightSensor {
    fn new(mqtt: MqttDeviceConfig, min: isize, max: isize) -> Self {
        let (tx, is_dark) = watch::channel(false);
        Self {
            is_dark,
            mqtt,
            min,
            max,
            tx,
        }
    }
}

pub async fn start(
    mut mqtt_rx: mqtt::Receiver,
    config: LightSensorConfig,
    client: AsyncClient,
) -> Result<Receiver, LightSensorError> {
    client
        .subscribe(config.mqtt.topic.clone(), rumqttc::QoS::AtLeastOnce)
        .await?;

    let mut light_sensor = LightSensor::new(config.mqtt, config.min, config.max);
    let is_dark = light_sensor.is_dark.clone();

    tokio::spawn(async move {
        loop {
            // TODO: Handle errors, warn if lagging
            if let Ok(message) = mqtt_rx.recv().await {
                light_sensor.on_mqtt(&message).await;
            }
        }
    });

    Ok(is_dark)
}

#[async_trait]
impl OnMqtt for LightSensor {
    fn topics(&self) -> Vec<&str> {
        vec![&self.mqtt.topic]
    }

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
            trace!(
                "In between min ({}) and max ({}) value, keeping current state: {}",
                self.min,
                self.max,
                *self.is_dark.borrow()
            );
            *self.is_dark.borrow()
        };

        if is_dark != *self.is_dark.borrow() {
            debug!("Dark state has changed: {is_dark}");
            self.tx.send(is_dark).ok();
        }
    }
}
