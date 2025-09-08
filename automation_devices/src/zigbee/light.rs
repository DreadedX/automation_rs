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
use google_home::traits::{Brightness, Color, ColorSetting, ColorTemperatureRange, OnOff};
use google_home::types::Type;
use rumqttc::{Publish, matches};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, trace, warn};

pub trait LightState:
    Debug + Clone + Default + Sync + Send + Serialize + Into<StateOnOff> + 'static
{
}

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct Config<T: LightState> {
    #[device_config(flatten)]
    pub info: InfoConfig,
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,

    #[device_config(from_lua, default)]
    pub callback: ActionCallback<(Light<T>, T)>,

    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, LuaSerialize)]
pub struct StateOnOff {
    #[serde(deserialize_with = "state_deserializer")]
    state: bool,
}

impl LightState for StateOnOff {}

#[derive(Debug, Clone, Default, Serialize, Deserialize, LuaSerialize)]
pub struct StateBrightness {
    #[serde(deserialize_with = "state_deserializer")]
    state: bool,
    brightness: f32,
}

impl LightState for StateBrightness {}

impl From<StateBrightness> for StateOnOff {
    fn from(state: StateBrightness) -> Self {
        StateOnOff { state: state.state }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, LuaSerialize)]
pub struct StateColorTemperature {
    #[serde(deserialize_with = "state_deserializer")]
    state: bool,
    brightness: f32,
    color_temp: u32,
}

impl LightState for StateColorTemperature {}

impl From<StateColorTemperature> for StateOnOff {
    fn from(state: StateColorTemperature) -> Self {
        StateOnOff { state: state.state }
    }
}

impl From<StateColorTemperature> for StateBrightness {
    fn from(state: StateColorTemperature) -> Self {
        StateBrightness {
            state: state.state,
            brightness: state.brightness,
        }
    }
}

#[derive(Debug, Clone, LuaDevice)]
#[traits(<StateOnOff>: OnOff)]
#[traits(<StateBrightness>: OnOff, Brightness)]
#[traits(<StateColorTemperature>: OnOff, Brightness, ColorSetting)]
pub struct Light<T: LightState> {
    config: Config<T>,

    state: Arc<RwLock<T>>,
}

pub type LightOnOff = Light<StateOnOff>;
pub type LightBrightness = Light<StateBrightness>;
pub type LightColorTemperature = Light<StateColorTemperature>;

impl<T: LightState> Light<T> {
    async fn state(&self) -> RwLockReadGuard<'_, T> {
        self.state.read().await
    }

    async fn state_mut(&self) -> RwLockWriteGuard<'_, T> {
        self.state.write().await
    }
}

#[async_trait]
impl<T: LightState> LuaDeviceCreate for Light<T> {
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

impl<T: LightState> Device for Light<T> {
    fn get_id(&self) -> String {
        self.config.info.identifier()
    }
}

#[async_trait]
impl OnMqtt for Light<StateOnOff> {
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
impl OnMqtt for Light<StateBrightness> {
    async fn on_mqtt(&self, message: Publish) {
        // Check if the message is from the deviec itself or from a remote
        if matches(&message.topic, &self.config.mqtt.topic) {
            let state = match serde_json::from_slice::<StateBrightness>(&message.payload) {
                Ok(state) => state,
                Err(err) => {
                    warn!(id = Device::get_id(self), "Failed to parse message: {err}");
                    return;
                }
            };

            {
                let current_state = self.state().await;
                // No need to do anything if the state has not changed
                if state.state == current_state.state
                    && state.brightness == current_state.brightness
                {
                    return;
                }
            }

            self.state_mut().await.state = state.state;
            self.state_mut().await.brightness = state.brightness;
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
impl OnMqtt for Light<StateColorTemperature> {
    async fn on_mqtt(&self, message: Publish) {
        // Check if the message is from the deviec itself or from a remote
        if matches(&message.topic, &self.config.mqtt.topic) {
            let state = match serde_json::from_slice::<StateColorTemperature>(&message.payload) {
                Ok(state) => state,
                Err(err) => {
                    warn!(id = Device::get_id(self), "Failed to parse message: {err}");
                    return;
                }
            };

            {
                let current_state = self.state().await;
                // No need to do anything if the state has not changed
                if state.state == current_state.state
                    && state.brightness == current_state.brightness
                    && state.color_temp == current_state.color_temp
                {
                    return;
                }
            }

            self.state_mut().await.state = state.state;
            self.state_mut().await.brightness = state.brightness;
            self.state_mut().await.color_temp = state.color_temp;
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
impl<T: LightState> google_home::Device for Light<T> {
    fn get_device_type(&self) -> Type {
        Type::Light
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
impl<T> OnOff for Light<T>
where
    T: LightState,
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

const FACTOR: f32 = 30.0;

#[async_trait]
impl<T> Brightness for Light<T>
where
    T: LightState,
    T: Into<StateBrightness>,
{
    async fn brightness(&self) -> Result<u8, ErrorCode> {
        let state = self.state().await;
        let state: StateBrightness = state.deref().clone().into();
        let brightness =
            100.0 * f32::log10(state.brightness / FACTOR + 1.0) / f32::log10(254.0 / FACTOR + 1.0);

        Ok(brightness.clamp(0.0, 100.0).round() as u8)
    }

    async fn set_brightness(&self, brightness: u8) -> Result<(), ErrorCode> {
        let brightness =
            FACTOR * ((FACTOR / (FACTOR + 254.0)).powf(-(brightness as f32) / 100.0) - 1.0);

        let message = json!({
            "brightness": brightness.clamp(0.0, 254.0).round() as u8
        });

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
impl<T> ColorSetting for Light<T>
where
    T: LightState,
    T: Into<StateColorTemperature>,
{
    fn color_temperature_range(&self) -> ColorTemperatureRange {
        ColorTemperatureRange {
            temperature_min_k: 2200,
            temperature_max_k: 4000,
        }
    }

    async fn color(&self) -> Color {
        let state = self.state().await;
        let state: StateColorTemperature = state.deref().clone().into();

        let temperature = 1_000_000 / state.color_temp;

        Color { temperature }
    }

    async fn set_color(&self, color: Color) -> Result<(), ErrorCode> {
        let temperature = 1_000_000 / color.temperature;

        let message = json!({
            "color_temp": temperature,
        });

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
