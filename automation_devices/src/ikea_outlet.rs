use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use automation_lib::action_callback::ActionCallback;
use automation_lib::config::{InfoConfig, MqttDeviceConfig};
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_lib::event::{OnMqtt, OnPresence};
use automation_lib::messages::OnOffMessage;
use automation_lib::mqtt::WrappedAsyncClient;
use automation_macro::LuaDeviceConfig;
use google_home::device;
use google_home::errors::ErrorCode;
use google_home::traits::{self, OnOff};
use google_home::types::Type;
use rumqttc::{matches, Publish};
use serde::Deserialize;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, error, trace, warn};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Copy)]
pub enum OutletType {
    Outlet,
    Kettle,
    Charger,
}

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct Config {
    #[device_config(flatten)]
    pub info: InfoConfig,
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    #[device_config(default(OutletType::Outlet))]
    pub outlet_type: OutletType,

    #[device_config(from_lua, default)]
    pub callback: ActionCallback<IkeaOutlet, bool>,

    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

#[derive(Debug, Default)]
pub struct State {
    last_known_state: bool,
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

        Ok(Self {
            config,
            state: Default::default(),
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

            self.config.callback.call(self, &state).await;

            debug!(id = Device::get_id(self), "Updating state to {state}");
            self.state_mut().await.last_known_state = state;
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

#[async_trait]
impl google_home::Device for IkeaOutlet {
    fn get_device_type(&self) -> Type {
        match self.config.outlet_type {
            OutletType::Outlet => Type::Outlet,
            OutletType::Kettle => Type::Kettle,
            OutletType::Charger => Type::Outlet, // Find a better device type for this, ideally would like to use charger, but that needs more work
        }
    }

    fn get_device_name(&self) -> device::Name {
        device::Name::new(&self.config.info.name)
    }

    fn get_id(&self) -> String {
        Device::get_id(self)
    }

    async fn is_online(&self) -> bool {
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
