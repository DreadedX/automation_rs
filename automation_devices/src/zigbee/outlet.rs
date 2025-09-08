use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use automation_lib::action_callback::ActionCallback;
use automation_lib::config::{InfoConfig, MqttDeviceConfig};
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_lib::event::OnMqtt;
use automation_lib::helpers::serialization::state_deserializer;
use automation_lib::mqtt::WrappedAsyncClient;
use automation_macro::{LuaDevice, LuaDeviceConfig, LuaSerialize};
use google_home::device;
use google_home::errors::ErrorCode;
use google_home::traits::OnOff;
use google_home::types::Type;
use rumqttc::{Publish, matches};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, trace, warn};

pub trait OutletState:
    Debug + Clone + Default + Sync + Send + Serialize + Into<StateOnOff> + 'static
{
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Copy)]
pub enum OutletType {
    Outlet,
    Kettle,
}

impl From<OutletType> for Type {
    fn from(outlet: OutletType) -> Self {
        match outlet {
            OutletType::Outlet => Type::Outlet,
            OutletType::Kettle => Type::Kettle,
        }
    }
}

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct Config<T: OutletState> {
    #[device_config(flatten)]
    pub info: InfoConfig,
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    #[device_config(default(OutletType::Outlet))]
    pub outlet_type: OutletType,

    #[device_config(from_lua, default)]
    pub callback: ActionCallback<(Outlet<T>, T)>,

    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, LuaSerialize)]
pub struct StateOnOff {
    #[serde(deserialize_with = "state_deserializer")]
    state: bool,
}

impl OutletState for StateOnOff {}

#[derive(Debug, Clone, Default, Serialize, Deserialize, LuaSerialize)]
pub struct StatePower {
    #[serde(deserialize_with = "state_deserializer")]
    state: bool,
    power: f64,
}

impl OutletState for StatePower {}

impl From<StatePower> for StateOnOff {
    fn from(state: StatePower) -> Self {
        StateOnOff { state: state.state }
    }
}

#[derive(Debug, Clone, LuaDevice)]
#[traits(<StateOnOff>: OnOff)]
#[traits(<StatePower>: OnOff)]
pub struct Outlet<T: OutletState> {
    config: Config<T>,

    state: Arc<RwLock<T>>,
}

pub type OutletOnOff = Outlet<StateOnOff>;
pub type OutletPower = Outlet<StatePower>;

impl<T: OutletState> Outlet<T> {
    async fn state(&self) -> RwLockReadGuard<'_, T> {
        self.state.read().await
    }

    async fn state_mut(&self) -> RwLockWriteGuard<'_, T> {
        self.state.write().await
    }
}

#[async_trait]
impl<T: OutletState> LuaDeviceCreate for Outlet<T> {
    type Config = Config<T>;
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

impl<T: OutletState> Device for Outlet<T> {
    fn get_id(&self) -> String {
        self.config.info.identifier()
    }
}

#[async_trait]
impl OnMqtt for Outlet<StateOnOff> {
    async fn on_mqtt(&self, message: Publish) {
        // Check if the message is from the device itself or from a remote
        if matches(&message.topic, &self.config.mqtt.topic) {
            let state = match serde_json::from_slice::<StateOnOff>(&message.payload) {
                Ok(state) => state,
                Err(err) => {
                    warn!(id = Device::get_id(self), "Failed to parse message: {err}");
                    return;
                }
            };

            // No need to do anything if the state has not changed
            if state.state == self.state().await.state {
                return;
            }

            self.state_mut().await.state = state.state;
            debug!(
                id = Device::get_id(self),
                "Updating state to {:?}",
                self.state().await
            );

            self.config
                .callback
                .call((self.clone(), self.state().await.clone()))
                .await;
        }
    }
}

#[async_trait]
impl OnMqtt for Outlet<StatePower> {
    async fn on_mqtt(&self, message: Publish) {
        // Check if the message is from the deviec itself or from a remote
        if matches(&message.topic, &self.config.mqtt.topic) {
            let state = match serde_json::from_slice::<StatePower>(&message.payload) {
                Ok(state) => state,
                Err(err) => {
                    warn!(id = Device::get_id(self), "Failed to parse message: {err}");
                    return;
                }
            };

            {
                let current_state = self.state().await;
                // No need to do anything if the state has not changed
                if state.state == current_state.state && state.power == current_state.power {
                    return;
                }
            }

            self.state_mut().await.state = state.state;
            self.state_mut().await.power = state.power;
            debug!(
                id = Device::get_id(self),
                "Updating state to {:?}",
                self.state().await
            );

            self.config
                .callback
                .call((self.clone(), self.state().await.clone()))
                .await;
        }
    }
}

#[async_trait]
impl<T: OutletState> google_home::Device for Outlet<T> {
    fn get_device_type(&self) -> Type {
        self.config.outlet_type.into()
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
impl<T> OnOff for Outlet<T>
where
    T: OutletState,
{
    async fn on(&self) -> Result<bool, ErrorCode> {
        let state = self.state().await;
        let state: StateOnOff = state.deref().clone().into();
        Ok(state.state)
    }

    async fn set_on(&self, on: bool) -> Result<(), ErrorCode> {
        let message = json!({
            "state": if on { "ON" } else { "OFF"}
        });

        debug!(id = Device::get_id(self), "{message}");

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
