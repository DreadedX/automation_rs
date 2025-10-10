use std::sync::Arc;

use async_trait::async_trait;
use automation_lib::action_callback::ActionCallback;
use automation_lib::config::{InfoConfig, MqttDeviceConfig};
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_lib::error::DeviceConfigError;
use automation_lib::event::OnMqtt;
use automation_lib::messages::ContactMessage;
use automation_lib::mqtt::WrappedAsyncClient;
use automation_macro::{Device, LuaDeviceConfig};
use google_home::device;
use google_home::errors::{DeviceError, ErrorCode};
use google_home::traits::OpenClose;
use google_home::types::Type;
use lua_typed::Typed;
use serde::Deserialize;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, error, trace};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Copy, Typed)]
pub enum SensorType {
    Door,
    Drawer,
    Window,
}
crate::register_type!(SensorType);

#[derive(Debug, Clone, LuaDeviceConfig, Typed)]
#[typed(as = "ContactSensorConfig")]
pub struct Config {
    #[device_config(flatten)]
    #[typed(flatten)]
    pub info: InfoConfig,
    #[device_config(flatten)]
    #[typed(flatten)]
    pub mqtt: MqttDeviceConfig,

    #[device_config(default(SensorType::Window))]
    #[typed(default)]
    pub sensor_type: SensorType,

    #[device_config(from_lua, default)]
    #[typed(default)]
    pub callback: ActionCallback<(ContactSensor, bool)>,
    #[device_config(from_lua, default)]
    #[typed(default)]
    pub battery_callback: ActionCallback<(ContactSensor, f32)>,

    #[device_config(from_lua)]
    #[typed(default)]
    pub client: WrappedAsyncClient,
}
crate::register_type!(Config);

#[derive(Debug)]
struct State {
    is_closed: bool,
}

#[derive(Debug, Clone, Device)]
#[device(traits(OpenClose))]
pub struct ContactSensor {
    config: Config,
    state: Arc<RwLock<State>>,
}
crate::register_device!(ContactSensor);

impl ContactSensor {
    async fn state(&self) -> RwLockReadGuard<'_, State> {
        self.state.read().await
    }

    async fn state_mut(&self) -> RwLockWriteGuard<'_, State> {
        self.state.write().await
    }
}

#[async_trait]
impl LuaDeviceCreate for ContactSensor {
    type Config = Config;
    type Error = DeviceConfigError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.info.identifier(), "Setting up ContactSensor");

        config
            .client
            .subscribe(&config.mqtt.topic, rumqttc::QoS::AtLeastOnce)
            .await?;

        let state = State { is_closed: true };
        let state = Arc::new(RwLock::new(state));

        Ok(Self { config, state })
    }
}

impl Device for ContactSensor {
    fn get_id(&self) -> String {
        self.config.info.identifier()
    }
}

#[async_trait]
impl google_home::Device for ContactSensor {
    fn get_device_type(&self) -> google_home::types::Type {
        match self.config.sensor_type {
            SensorType::Door => Type::Door,
            SensorType::Drawer => Type::Drawer,
            SensorType::Window => Type::Window,
        }
    }

    fn get_id(&self) -> String {
        Device::get_id(self)
    }

    fn get_device_name(&self) -> google_home::device::Name {
        device::Name::new(&self.config.info.name)
    }

    fn get_room_hint(&self) -> Option<&str> {
        self.config.info.room.as_deref()
    }

    fn will_report_state(&self) -> bool {
        false
    }

    async fn is_online(&self) -> bool {
        true
    }
}

#[async_trait]
impl OpenClose for ContactSensor {
    fn discrete_only_open_close(&self) -> Option<bool> {
        Some(true)
    }

    fn query_only_open_close(&self) -> Option<bool> {
        Some(true)
    }

    async fn open_percent(&self) -> Result<u8, ErrorCode> {
        if self.state().await.is_closed {
            Ok(0)
        } else {
            Ok(100)
        }
    }

    async fn set_open_percent(&self, _open_percent: u8) -> Result<(), ErrorCode> {
        Err(DeviceError::ActionNotAvailable.into())
    }
}

#[async_trait]
impl OnMqtt for ContactSensor {
    async fn on_mqtt(&self, message: rumqttc::Publish) {
        if !rumqttc::matches(&message.topic, &self.config.mqtt.topic) {
            return;
        }

        let message = match ContactMessage::try_from(message) {
            Ok(message) => message,
            Err(err) => {
                error!(id = self.get_id(), "Failed to parse message: {err}");
                return;
            }
        };

        if let Some(is_closed) = message.contact {
            if is_closed == self.state().await.is_closed {
                return;
            }

            self.config.callback.call((self.clone(), !is_closed)).await;

            debug!(id = self.get_id(), "Updating state to {is_closed}");
            self.state_mut().await.is_closed = is_closed;
        }

        if let Some(battery) = message.battery {
            self.config
                .battery_callback
                .call((self.clone(), battery))
                .await;
        }
    }
}
