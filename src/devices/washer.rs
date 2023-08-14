use async_trait::async_trait;
use rumqttc::{AsyncClient, Publish};
use serde::Deserialize;
use tracing::{error, warn};

use crate::{
    config::{CreateDevice, MqttDeviceConfig},
    device_manager::DeviceManager,
    error::CreateDeviceError,
    event::{Event, EventChannel, OnMqtt},
    messages::PowerMessage,
};

use super::{ntfy::Priority, Device, Notification};

#[derive(Debug, Clone, Deserialize)]
pub struct WasherConfig {
    #[serde(flatten)]
    mqtt: MqttDeviceConfig,
}

// TODO: Add google home integration

#[derive(Debug)]
pub struct Washer {
    identifier: String,
    mqtt: MqttDeviceConfig,

    event_channel: EventChannel,
    running: bool,
}

#[async_trait]
impl CreateDevice for Washer {
    type Config = WasherConfig;

    async fn create(
        identifier: &str,
        config: Self::Config,
        event_channel: &EventChannel,
        _client: &AsyncClient,
        _presence_topic: &str,
        _device_manager: &DeviceManager,
    ) -> Result<Self, CreateDeviceError> {
        Ok(Self {
            identifier: identifier.to_owned(),
            mqtt: config.mqtt,
            event_channel: event_channel.clone(),
            running: false,
        })
    }
}

impl Device for Washer {
    fn get_id(&self) -> &str {
        &self.identifier
    }
}

#[async_trait]
impl OnMqtt for Washer {
    fn topics(&self) -> Vec<&str> {
        vec![&self.mqtt.topic]
    }

    async fn on_mqtt(&mut self, message: Publish) {
        let power = match PowerMessage::try_from(message) {
            Ok(state) => state.power(),
            Err(err) => {
                error!(id = self.identifier, "Failed to parse message: {err}");
                return;
            }
        };

        if self.running && power < 1.0 {
            // The washer is done running
            self.running = false;
            let notification = Notification::new()
                .set_title("Laundy is done")
                .set_message("Don't forget to hang it!")
                .add_tag("womans_clothes")
                .set_priority(Priority::High);

            if self
                .event_channel
                .get_tx()
                .send(Event::Ntfy(notification))
                .await
                .is_err()
            {
                warn!("There are no receivers on the event channel");
            }
        } else if !self.running && power >= 1.0 {
            // We just started washing
            self.running = true
        }
    }
}
