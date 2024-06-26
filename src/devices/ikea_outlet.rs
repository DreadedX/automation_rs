use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use automation_macro::{LuaDevice, LuaDeviceConfig};
use google_home::errors::ErrorCode;
use google_home::traits::{self, OnOff};
use google_home::types::Type;
use google_home::{device, GoogleHomeDevice};
use rumqttc::{matches, Publish, SubscribeFilter};
use serde::Deserialize;
use tokio::task::JoinHandle;
use tracing::{debug, error, trace, warn};

use super::LuaDeviceCreate;
use crate::config::{InfoConfig, MqttDeviceConfig};
use crate::devices::Device;
use crate::event::{OnMqtt, OnPresence};
use crate::messages::{OnOffMessage, RemoteAction, RemoteMessage};
use crate::mqtt::WrappedAsyncClient;
use crate::traits::Timeout;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Copy)]
pub enum OutletType {
    Outlet,
    Kettle,
    Charger,
    Light,
}

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct IkeaOutletConfig {
    #[device_config(flatten)]
    pub info: InfoConfig,
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    #[device_config(default(OutletType::Outlet))]
    pub outlet_type: OutletType,
    #[device_config(default, with(|t: Option<_>| t.map(Duration::from_secs)))]
    pub timeout: Option<Duration>,
    #[device_config(default)]
    pub remotes: Vec<MqttDeviceConfig>,

    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

#[derive(Debug, LuaDevice)]
pub struct IkeaOutlet {
    config: IkeaOutletConfig,

    last_known_state: bool,
    handle: Option<JoinHandle<()>>,
}

async fn set_on(client: WrappedAsyncClient, topic: &str, on: bool) {
    let message = OnOffMessage::new(on);

    let topic = format!("{}/set", topic);
    // TODO: Handle potential errors here
    client
        .publish(
            &topic,
            rumqttc::QoS::AtLeastOnce,
            false,
            serde_json::to_string(&message).unwrap(),
        )
        .await
        .map_err(|err| warn!("Failed to update state on {topic}: {err}"))
        .ok();
}

#[async_trait]
impl LuaDeviceCreate for IkeaOutlet {
    type Config = IkeaOutletConfig;
    type Error = rumqttc::ClientError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.info.identifier(), "Setting up IkeaOutlet");

        if !config.remotes.is_empty() {
            config
                .client
                .subscribe_many(config.remotes.iter().map(|remote| SubscribeFilter {
                    path: remote.topic.clone(),
                    qos: rumqttc::QoS::AtLeastOnce,
                }))
                .await?;
        }

        config
            .client
            .subscribe(&config.mqtt.topic, rumqttc::QoS::AtLeastOnce)
            .await?;

        Ok(Self {
            config,
            last_known_state: false,
            handle: None,
        })
    }
}

impl Device for IkeaOutlet {
    fn get_id(&self) -> String {
        self.config.info.identifier()
    }
}

#[async_trait]
impl OnMqtt for IkeaOutlet {
    async fn on_mqtt(&mut self, message: Publish) {
        // Check if the message is from the deviec itself or from a remote
        if matches(&message.topic, &self.config.mqtt.topic) {
            // Update the internal state based on what the device has reported
            let state = match OnOffMessage::try_from(message) {
                Ok(state) => state.state(),
                Err(err) => {
                    error!(id = Device::get_id(self), "Failed to parse message: {err}");
                    return;
                }
            };

            // No need to do anything if the state has not changed
            if state == self.last_known_state {
                return;
            }

            // Abort any timer that is currently running
            self.stop_timeout().await.unwrap();

            debug!(id = Device::get_id(self), "Updating state to {state}");
            self.last_known_state = state;

            // If this is a kettle start a timeout for turning it of again
            if state && let Some(timeout) = self.config.timeout {
                self.start_timeout(timeout).await.unwrap();
            }
        } else if self
            .config
            .remotes
            .iter()
            .any(|remote| rumqttc::matches(&message.topic, &remote.topic))
        {
            let action = match RemoteMessage::try_from(message) {
                Ok(message) => message.action(),
                Err(err) => {
                    error!(id = Device::get_id(self), "Failed to parse message: {err}");
                    return;
                }
            };

            match action {
				RemoteAction::On => self.set_on(true).await.unwrap(),
				RemoteAction::BrightnessMoveUp => self.set_on(false).await.unwrap(),
				RemoteAction::BrightnessStop => { /* Ignore this action */ },
				_ => warn!("Expected ikea shortcut button which only supports 'on' and 'brightness_move_up', got: {action:?}")
			}
        }
    }
}

#[async_trait]
impl OnPresence for IkeaOutlet {
    async fn on_presence(&mut self, presence: bool) {
        // Turn off the outlet when we leave the house (Not if it is a battery charger)
        if !presence && self.config.outlet_type != OutletType::Charger {
            debug!(id = Device::get_id(self), "Turning device off");
            self.set_on(false).await.ok();
        }
    }
}

impl GoogleHomeDevice for IkeaOutlet {
    fn get_device_type(&self) -> Type {
        match self.config.outlet_type {
            OutletType::Outlet => Type::Outlet,
            OutletType::Kettle => Type::Kettle,
            OutletType::Light => Type::Light, // Find a better device type for this, ideally would like to use charger, but that needs more work
            OutletType::Charger => Type::Outlet, // Find a better device type for this, ideally would like to use charger, but that needs more work
        }
    }

    fn get_device_name(&self) -> device::Name {
        device::Name::new(&self.config.info.name)
    }

    fn get_id(&self) -> String {
        Device::get_id(self)
    }

    fn is_online(&self) -> bool {
        true
    }

    fn get_room_hint(&self) -> Option<&str> {
        self.config.info.room.as_deref()
    }

    fn will_report_state(&self) -> bool {
        // TODO: Implement state reporting
        false
    }
}

#[async_trait]
impl traits::OnOff for IkeaOutlet {
    async fn is_on(&self) -> Result<bool, ErrorCode> {
        Ok(self.last_known_state)
    }

    async fn set_on(&mut self, on: bool) -> Result<(), ErrorCode> {
        set_on(self.config.client.clone(), &self.config.mqtt.topic, on).await;

        Ok(())
    }
}

#[async_trait]
impl crate::traits::Timeout for IkeaOutlet {
    async fn start_timeout(&mut self, timeout: Duration) -> Result<()> {
        // Abort any timer that is currently running
        self.stop_timeout().await?;

        // Turn the kettle of after the specified timeout
        // TODO: Impl Drop for IkeaOutlet that will abort the handle if the IkeaOutlet
        // get dropped
        let client = self.config.client.clone();
        let topic = self.config.mqtt.topic.clone();
        let id = Device::get_id(self).clone();
        self.handle = Some(tokio::spawn(async move {
            debug!(id, "Starting timeout ({timeout:?})...");
            tokio::time::sleep(timeout).await;
            debug!(id, "Turning outlet off!");
            // TODO: Idealy we would call self.set_on(false), however since we want to do
            // it after a timeout we have to put it in a separate task.
            // I don't think we can really get around calling outside function
            set_on(client, &topic, false).await;
        }));

        Ok(())
    }

    async fn stop_timeout(&mut self) -> Result<()> {
        if let Some(handle) = self.handle.take() {
            handle.abort();
        }

        Ok(())
    }
}
