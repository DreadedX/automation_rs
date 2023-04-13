use async_trait::async_trait;
use rumqttc::{matches, AsyncClient};
use serde::Deserialize;
use tracing::{debug, error, trace, warn};

use crate::{
    config::MqttDeviceConfig,
    error::LightSensorError,
    event::{Event, EventChannel},
    mqtt::BrightnessMessage,
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

const DEFAULT: bool = false;

pub async fn start(
    config: LightSensorConfig,
    event_channel: &EventChannel,
    client: AsyncClient,
) -> Result<(), LightSensorError> {
    // Subscrive to the mqtt topic
    client
        .subscribe(config.mqtt.topic.clone(), rumqttc::QoS::AtLeastOnce)
        .await?;

    // Create the channels
    let mut rx = event_channel.get_rx();
    let tx = event_channel.get_tx();

    // Setup default value, this is needed for hysteresis
    let mut current_is_dark = DEFAULT;

    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(Event::MqttMessage(message)) => {
                    if !matches(&message.topic, &config.mqtt.topic) {
                        continue;
                    }

                    let illuminance = match BrightnessMessage::try_from(message) {
                        Ok(state) => state.illuminance(),
                        Err(err) => {
                            error!("Failed to parse message: {err}");
                            continue;
                        }
                    };

                    debug!("Illuminance: {illuminance}");
                    let is_dark = if illuminance <= config.min {
                        trace!("It is dark");
                        true
                    } else if illuminance >= config.max {
                        trace!("It is light");
                        false
                    } else {
                        trace!(
                            "In between min ({}) and max ({}) value, keeping current state: {}",
                            config.min,
                            config.max,
                            current_is_dark
                        );
                        current_is_dark
                    };

                    if is_dark != current_is_dark {
                        debug!("Dark state has changed: {is_dark}");
                        current_is_dark = is_dark;

                        if tx.send(Event::Darkness(is_dark)).is_err() {
                            warn!("There are no receivers on the event channel");
                        }
                    }
                }
                Ok(_) => {}
                Err(_) => todo!("Handle errors with the event channel properly"),
            }
        }
    });

    Ok(())
}
