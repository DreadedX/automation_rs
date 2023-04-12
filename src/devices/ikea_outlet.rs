use async_trait::async_trait;
use google_home::errors::ErrorCode;
use google_home::{
    device,
    traits::{self, OnOff},
    types::Type,
    GoogleHomeDevice,
};
use pollster::FutureExt as _;
use rumqttc::{matches, AsyncClient, Publish};
use std::time::Duration;
use tokio::task::JoinHandle;
use tracing::{debug, error, warn};

use crate::config::{InfoConfig, MqttDeviceConfig, OutletType};
use crate::devices::Device;
use crate::mqtt::{OnMqtt, OnOffMessage};
use crate::presence::OnPresence;

#[derive(Debug)]
pub struct IkeaOutlet {
    identifier: String,
    info: InfoConfig,
    mqtt: MqttDeviceConfig,
    outlet_type: OutletType,
    timeout: Option<u64>,

    client: AsyncClient,
    last_known_state: bool,
    handle: Option<JoinHandle<()>>,
}

impl IkeaOutlet {
    pub fn new(
        identifier: &str,
        info: InfoConfig,
        mqtt: MqttDeviceConfig,
        outlet_type: OutletType,
        timeout: Option<u64>,
        client: AsyncClient,
    ) -> Self {
        Self {
            identifier: identifier.to_owned(),
            info,
            mqtt,
            outlet_type,
            timeout,
            client,
            last_known_state: false,
            handle: None,
        }
    }
}

async fn set_on(client: AsyncClient, topic: &str, on: bool) {
    let message = OnOffMessage::new(on);

    let topic = format!("{}/set", topic);
    // TODO: Handle potential errors here
    client
        .publish(
            topic.clone(),
            rumqttc::QoS::AtLeastOnce,
            false,
            serde_json::to_string(&message).unwrap(),
        )
        .await
        .map_err(|err| warn!("Failed to update state on {topic}: {err}"))
        .ok();
}

impl Device for IkeaOutlet {
    fn get_id(&self) -> &str {
        &self.identifier
    }
}

#[async_trait]
impl OnMqtt for IkeaOutlet {
    fn topics(&self) -> Vec<&str> {
        vec![&self.mqtt.topic]
    }

    async fn on_mqtt(&mut self, message: &Publish) {
        // Update the internal state based on what the device has reported
        if !matches(&message.topic, &self.mqtt.topic) {
            return;
        }

        let state = match OnOffMessage::try_from(message) {
            Ok(state) => state.state(),
            Err(err) => {
                error!(id = self.identifier, "Failed to parse message: {err}");
                return;
            }
        };

        // No need to do anything if the state has not changed
        if state == self.last_known_state {
            return;
        }

        // Abort any timer that is currently running
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }

        debug!(id = self.identifier, "Updating state to {state}");
        self.last_known_state = state;

        // If this is a kettle start a timeout for turning it of again
        if state {
            let timeout = match self.timeout {
                Some(timeout) => Duration::from_secs(timeout),
                None => return,
            };

            // Turn the kettle of after the specified timeout
            // TODO: Impl Drop for IkeaOutlet that will abort the handle if the IkeaOutlet
            // get dropped
            let client = self.client.clone();
            let topic = self.mqtt.topic.clone();
            let id = self.identifier.clone();
            self.handle = Some(tokio::spawn(async move {
                debug!(id, "Starting timeout ({timeout:?})...");
                tokio::time::sleep(timeout).await;
                debug!(id, "Turning outlet off!");
                // TODO: Idealy we would call self.set_on(false), however since we want to do
                // it after a timeout we have to put it in a seperate task.
                // I don't think we can really get around calling outside function
                set_on(client, &topic, false).await;
            }));
        }
    }
}

#[async_trait]
impl OnPresence for IkeaOutlet {
    async fn on_presence(&mut self, presence: bool) {
        // Turn off the outlet when we leave the house (Not if it is a battery charger)
        if !presence && self.outlet_type != OutletType::Charger {
            debug!(id = self.identifier, "Turning device off");
            self.set_on(false).ok();
        }
    }
}

impl GoogleHomeDevice for IkeaOutlet {
    fn get_device_type(&self) -> Type {
        match self.outlet_type {
            OutletType::Outlet => Type::Outlet,
            OutletType::Kettle => Type::Kettle,
            OutletType::Charger => Type::Outlet, // Find a better device type for this, ideally would like to use charger, but that needs more work
        }
    }

    fn get_device_name(&self) -> device::Name {
        device::Name::new(&self.info.name)
    }

    fn get_id(&self) -> &str {
        Device::get_id(self)
    }

    fn is_online(&self) -> bool {
        true
    }

    fn get_room_hint(&self) -> Option<&str> {
        self.info.room.as_deref()
    }

    fn will_report_state(&self) -> bool {
        // TODO: Implement state reporting
        false
    }
}

impl traits::OnOff for IkeaOutlet {
    fn is_on(&self) -> Result<bool, ErrorCode> {
        Ok(self.last_known_state)
    }

    fn set_on(&mut self, on: bool) -> Result<(), ErrorCode> {
        set_on(self.client.clone(), &self.mqtt.topic, on).block_on();

        Ok(())
    }
}
