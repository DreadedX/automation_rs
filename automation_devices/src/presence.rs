use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use automation_lib::action_callback::ActionCallback;
use automation_lib::config::MqttDeviceConfig;
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_lib::event::OnMqtt;
use automation_lib::lua::traits::AddAdditionalMethods;
use automation_lib::messages::PresenceMessage;
use automation_lib::mqtt::WrappedAsyncClient;
use automation_macro::{LuaDevice, LuaDeviceConfig};
use rumqttc::Publish;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, trace, warn};

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct Config {
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,

    #[device_config(from_lua, default)]
    pub callback: ActionCallback<Presence, bool>,

    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

pub const DEFAULT_PRESENCE: bool = false;

#[derive(Debug)]
pub struct State {
    devices: HashMap<String, bool>,
    current_overall_presence: bool,
}

#[derive(Debug, Clone, LuaDevice)]
#[traits(AddAdditionalMethods)]
pub struct Presence {
    config: Config,
    state: Arc<RwLock<State>>,
}

impl Presence {
    async fn state(&self) -> RwLockReadGuard<'_, State> {
        self.state.read().await
    }

    async fn state_mut(&self) -> RwLockWriteGuard<'_, State> {
        self.state.write().await
    }
}

#[async_trait]
impl LuaDeviceCreate for Presence {
    type Config = Config;
    type Error = rumqttc::ClientError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = "presence", "Setting up Presence");

        config
            .client
            .subscribe(&config.mqtt.topic, rumqttc::QoS::AtLeastOnce)
            .await?;

        let state = State {
            devices: HashMap::new(),
            current_overall_presence: DEFAULT_PRESENCE,
        };
        let state = Arc::new(RwLock::new(state));

        Ok(Self { config, state })
    }
}

impl Device for Presence {
    fn get_id(&self) -> String {
        "presence".to_string()
    }
}

#[async_trait]
impl OnMqtt for Presence {
    async fn on_mqtt(&self, message: Publish) {
        if !rumqttc::matches(&message.topic, &self.config.mqtt.topic) {
            return;
        }

        let offset = self
            .config
            .mqtt
            .topic
            .find('+')
            .or(self.config.mqtt.topic.find('#'))
            .expect("Presence::create fails if it does not contain wildcards");
        let device_name = message.topic[offset..].into();

        if message.payload.is_empty() {
            // Remove the device from the map
            debug!("State of device [{device_name}] has been removed");
            self.state_mut().await.devices.remove(&device_name);
        } else {
            let present = match PresenceMessage::try_from(message) {
                Ok(state) => state.presence(),
                Err(err) => {
                    warn!("Failed to parse message: {err}");
                    return;
                }
            };

            debug!("State of device [{device_name}] has changed: {}", present);
            self.state_mut().await.devices.insert(device_name, present);
        }

        let overall_presence = self.state().await.devices.iter().any(|(_, v)| *v);
        if overall_presence != self.state().await.current_overall_presence {
            debug!("Overall presence updated: {overall_presence}");
            self.state_mut().await.current_overall_presence = overall_presence;

            self.config.callback.call(self, &overall_presence).await;
        }
    }
}

impl AddAdditionalMethods for Presence {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M)
    where
        Self: Sized + 'static,
    {
        methods.add_async_method("overall_presence", async |_lua, this, ()| {
            Ok(this.state().await.current_overall_presence)
        });
    }
}
