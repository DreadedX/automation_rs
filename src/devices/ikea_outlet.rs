use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use automation_macro::LuaDeviceConfig;
use google_home::device;
use google_home::errors::ErrorCode;
use google_home::traits::{self, OnOff};
use google_home::types::Type;
use rumqttc::{matches, Publish};
use serde::Deserialize;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio::task::JoinHandle;
use tracing::{debug, error, trace, warn};

use super::LuaDeviceCreate;
use crate::config::{InfoConfig, MqttDeviceConfig};
use crate::devices::Device;
use crate::event::{OnMqtt, OnPresence};
use crate::messages::OnOffMessage;
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
pub struct Config {
    #[device_config(flatten)]
    pub info: InfoConfig,
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    #[device_config(default(OutletType::Outlet))]
    pub outlet_type: OutletType,
    #[device_config(default, with(|t: Option<_>| t.map(Duration::from_secs)))]
    pub timeout: Option<Duration>,

    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

#[derive(Debug)]
pub struct State {
    last_known_state: bool,
    handle: Option<JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub struct IkeaOutlet {
    config: Config,

    state: Arc<RwLock<State>>,
}

impl IkeaOutlet {
    async fn state(&self) -> RwLockReadGuard<State> {
        self.state.read().await
    }

    async fn state_mut(&self) -> RwLockWriteGuard<State> {
        self.state.write().await
    }
}

#[async_trait]
impl LuaDeviceCreate for IkeaOutlet {
    type Config = Config;
    type Error = rumqttc::ClientError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.info.identifier(), "Setting up IkeaOutlet");

        config
            .client
            .subscribe(&config.mqtt.topic, rumqttc::QoS::AtLeastOnce)
            .await?;

        let state = State {
            last_known_state: false,
            handle: None,
        };
        let state = Arc::new(RwLock::new(state));

        Ok(Self { config, state })
    }
}

impl Device for IkeaOutlet {
    fn get_id(&self) -> String {
        self.config.info.identifier()
    }
}

#[async_trait]
impl OnMqtt for IkeaOutlet {
    async fn on_mqtt(&self, message: Publish) {
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
            if state == self.state().await.last_known_state {
                return;
            }

            // Abort any timer that is currently running
            self.stop_timeout().await.unwrap();

            debug!(id = Device::get_id(self), "Updating state to {state}");
            self.state_mut().await.last_known_state = state;

            // If this is a kettle start a timeout for turning it of again
            if state && let Some(timeout) = self.config.timeout {
                self.start_timeout(timeout).await.unwrap();
            }
        }
    }
}

#[async_trait]
impl OnPresence for IkeaOutlet {
    async fn on_presence(&self, presence: bool) {
        // Turn off the outlet when we leave the house (Not if it is a battery charger)
        if !presence && self.config.outlet_type != OutletType::Charger {
            debug!(id = Device::get_id(self), "Turning device off");
            self.set_on(false).await.ok();
        }
    }
}

impl google_home::Device for IkeaOutlet {
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
    async fn on(&self) -> Result<bool, ErrorCode> {
        Ok(self.state().await.last_known_state)
    }

    async fn set_on(&self, on: bool) -> Result<(), ErrorCode> {
        let message = OnOffMessage::new(on);

        let topic = format!("{}/set", self.config.mqtt.topic);
        // TODO: Handle potential errors here
        self.config
            .client
            .publish(
                &topic,
                rumqttc::QoS::AtLeastOnce,
                false,
                serde_json::to_string(&message).unwrap(),
            )
            .await
            .map_err(|err| warn!("Failed to update state on {topic}: {err}"))
            .ok();

        Ok(())
    }
}

#[async_trait]
impl crate::traits::Timeout for IkeaOutlet {
    async fn start_timeout(&self, timeout: Duration) -> Result<()> {
        // Abort any timer that is currently running
        self.stop_timeout().await?;

        let device = self.clone();
        self.state_mut().await.handle = Some(tokio::spawn(async move {
            debug!(id = device.get_id(), "Starting timeout ({timeout:?})...");
            tokio::time::sleep(timeout).await;
            debug!(id = device.get_id(), "Turning outlet off!");
            device.set_on(false).await.unwrap();
        }));

        Ok(())
    }

    async fn stop_timeout(&self) -> Result<()> {
        if let Some(handle) = self.state_mut().await.handle.take() {
            handle.abort();
        }

        Ok(())
    }
}
