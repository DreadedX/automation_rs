use async_trait::async_trait;
use automation_macro::{LuaDevice, LuaDeviceConfig};
use rumqttc::Publish;
use tracing::{debug, error, warn};

use super::ntfy::Priority;
use super::{Device, Notification};
use crate::config::MqttDeviceConfig;
use crate::device_manager::DeviceConfig;
use crate::error::DeviceConfigError;
use crate::event::{self, Event, EventChannel, OnMqtt};
use crate::messages::PowerMessage;

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct WasherConfig {
    #[device_config(flatten)]
    mqtt: MqttDeviceConfig,
    // Power in Watt
    threshold: f32,
    #[device_config(rename("event_channel"), from_lua, with(|ec: EventChannel| ec.get_tx()))]
    pub tx: event::Sender,
}

#[async_trait]
impl DeviceConfig for WasherConfig {
    async fn create(&self, identifier: &str) -> Result<Box<dyn Device>, DeviceConfigError> {
        let device = Washer {
            identifier: identifier.into(),
            config: self.clone(),
            running: 0,
        };

        Ok(Box::new(device))
    }
}

// TODO: Add google home integration

#[derive(Debug, LuaDevice)]
pub struct Washer {
    identifier: String,
    #[config]
    config: WasherConfig,

    running: isize,
}

impl Device for Washer {
    fn get_id(&self) -> &str {
        &self.identifier
    }
}

// The washer needs to have a power draw above the theshold multiple times before the washer is
// actually marked as running
// This helps prevent false positives
const HYSTERESIS: isize = 10;

#[async_trait]
impl OnMqtt for Washer {
    fn topics(&self) -> Vec<&str> {
        vec![&self.config.mqtt.topic]
    }

    async fn on_mqtt(&mut self, message: Publish) {
        let power = match PowerMessage::try_from(message) {
            Ok(state) => state.power(),
            Err(err) => {
                error!(id = self.identifier, "Failed to parse message: {err}");
                return;
            }
        };

        // debug!(id = self.identifier, power, "Washer state update");

        if power < self.config.threshold && self.running >= HYSTERESIS {
            // The washer is done running
            debug!(
                id = self.identifier,
                power,
                threshold = self.config.threshold,
                "Washer is done"
            );

            self.running = 0;
            let notification = Notification::new()
                .set_title("Laundy is done")
                .set_message("Don't forget to hang it!")
                .add_tag("womans_clothes")
                .set_priority(Priority::High);

            if self
                .config
                .tx
                .send(Event::Ntfy(notification))
                .await
                .is_err()
            {
                warn!("There are no receivers on the event channel");
            }
        } else if power < self.config.threshold {
            // Prevent false positives
            self.running = 0;
        } else if power >= self.config.threshold && self.running < HYSTERESIS {
            // Washer could be starting
            debug!(
                id = self.identifier,
                power,
                threshold = self.config.threshold,
                "Washer is starting"
            );

            self.running += 1;
        }
    }
}
