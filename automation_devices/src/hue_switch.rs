use async_trait::async_trait;
use automation_lib::action_callback::ActionCallback;
use automation_lib::config::{InfoConfig, MqttDeviceConfig};
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_lib::event::OnMqtt;
use automation_lib::mqtt::WrappedAsyncClient;
use automation_macro::LuaDeviceConfig;
use rumqttc::{matches, Publish};
use serde::Deserialize;
use tracing::{debug, trace, warn};

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct Config {
    #[device_config(flatten)]
    pub info: InfoConfig,

    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,

    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,

    #[device_config(from_lua, default)]
    pub left_callback: ActionCallback<HueSwitch, ()>,

    #[device_config(from_lua, default)]
    pub right_callback: ActionCallback<HueSwitch, ()>,

    #[device_config(from_lua, default)]
    pub left_hold_callback: ActionCallback<HueSwitch, ()>,

    #[device_config(from_lua, default)]
    pub right_hold_callback: ActionCallback<HueSwitch, ()>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Action {
    LeftPress,
    LeftPressRelease,
    LeftHold,
    LeftHoldRelease,
    RightPress,
    RightPressRelease,
    RightHold,
    RightHoldRelease,
}

#[derive(Debug, Clone, Deserialize)]
struct State {
    action: Action,
}

#[derive(Debug, Clone)]
pub struct HueSwitch {
    config: Config,
}

impl Device for HueSwitch {
    fn get_id(&self) -> String {
        self.config.info.identifier()
    }
}

#[async_trait]
impl LuaDeviceCreate for HueSwitch {
    type Config = Config;
    type Error = rumqttc::ClientError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.info.identifier(), "Setting up HueSwitch");

        config
            .client
            .subscribe(&config.mqtt.topic, rumqttc::QoS::AtLeastOnce)
            .await?;

        Ok(Self { config })
    }
}

#[async_trait]
impl OnMqtt for HueSwitch {
    async fn on_mqtt(&self, message: Publish) {
        // Check if the message is from the device itself or from a remote
        if matches(&message.topic, &self.config.mqtt.topic) {
            let action = match serde_json::from_slice::<State>(&message.payload) {
                Ok(message) => message.action,
                Err(err) => {
                    warn!(id = Device::get_id(self), "Failed to parse message: {err}");
                    return;
                }
            };
            debug!(id = Device::get_id(self), "Remote action = {:?}", action);

            match action {
                Action::LeftPressRelease => self.config.left_callback.call(self, &()).await,
                Action::LeftHold => self.config.left_hold_callback.call(self, &()).await,
                Action::RightPressRelease => self.config.right_callback.call(self, &()).await,
                Action::RightHold => self.config.right_hold_callback.call(self, &()).await,
                _ => {}
            }
        }
    }
}
