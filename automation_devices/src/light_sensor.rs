use std::sync::Arc;

use async_trait::async_trait;
use automation_lib::action_callback::ActionCallback;
use automation_lib::config::MqttDeviceConfig;
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_lib::event::OnMqtt;
use automation_lib::messages::BrightnessMessage;
use automation_lib::mqtt::WrappedAsyncClient;
use automation_macro::{Device, LuaDeviceConfig};
use rumqttc::Publish;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, trace, warn};

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct Config {
    pub identifier: String,
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    pub min: isize,
    pub max: isize,

    #[device_config(from_lua, default)]
    pub callback: ActionCallback<(LightSensor, bool)>,

    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

const DEFAULT: bool = false;

#[derive(Debug)]
pub struct State {
    is_dark: bool,
}

#[derive(Debug, Clone, Device)]
pub struct LightSensor {
    config: Config,
    state: Arc<RwLock<State>>,
}

impl LightSensor {
    async fn state(&self) -> RwLockReadGuard<'_, State> {
        self.state.read().await
    }

    async fn state_mut(&self) -> RwLockWriteGuard<'_, State> {
        self.state.write().await
    }
}

#[async_trait]
impl LuaDeviceCreate for LightSensor {
    type Config = Config;
    type Error = rumqttc::ClientError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.identifier, "Setting up LightSensor");

        config
            .client
            .subscribe(&config.mqtt.topic, rumqttc::QoS::AtLeastOnce)
            .await?;

        let state = State { is_dark: DEFAULT };
        let state = Arc::new(RwLock::new(state));

        Ok(Self { config, state })
    }
}

impl Device for LightSensor {
    fn get_id(&self) -> String {
        self.config.identifier.clone()
    }
}

#[async_trait]
impl OnMqtt for LightSensor {
    async fn on_mqtt(&self, message: Publish) {
        if !rumqttc::matches(&message.topic, &self.config.mqtt.topic) {
            return;
        }

        let illuminance = match BrightnessMessage::try_from(message) {
            Ok(state) => state.illuminance(),
            Err(err) => {
                warn!("Failed to parse message: {err}");
                return;
            }
        };

        // TODO: Move this logic to lua at some point
        debug!("Illuminance: {illuminance}");
        let is_dark = if illuminance <= self.config.min {
            trace!("It is dark");
            true
        } else if illuminance >= self.config.max {
            trace!("It is light");
            false
        } else {
            let is_dark = self.state().await.is_dark;
            trace!(
                "In between min ({}) and max ({}) value, keeping current state: {}",
                self.config.min, self.config.max, is_dark
            );
            is_dark
        };

        if is_dark != self.state().await.is_dark {
            debug!("Dark state has changed: {is_dark}");
            self.state_mut().await.is_dark = is_dark;

            self.config
                .callback
                .call((self.clone(), !self.state().await.is_dark))
                .await;
        }
    }
}
