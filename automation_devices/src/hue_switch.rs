use async_trait::async_trait;
use automation_lib::action_callback::ActionCallback;
use automation_lib::config::{InfoConfig, MqttDeviceConfig};
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_lib::event::OnMqtt;
use automation_lib::mqtt::WrappedAsyncClient;
use automation_macro::{LuaDevice, LuaDeviceConfig};
use rumqttc::{Publish, matches};
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
    pub left_callback: ActionCallback<HueSwitch>,

    #[device_config(from_lua, default)]
    pub right_callback: ActionCallback<HueSwitch>,

    #[device_config(from_lua, default)]
    pub left_hold_callback: ActionCallback<HueSwitch>,

    #[device_config(from_lua, default)]
    pub right_hold_callback: ActionCallback<HueSwitch>,

    #[device_config(from_lua, default)]
    pub battery_callback: ActionCallback<(HueSwitch, f32)>,
}

#[derive(Debug, Copy, Clone, Deserialize)]
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
    action: Option<Action>,
    battery: Option<f32>,
}

#[derive(Debug, Clone, LuaDevice)]
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
            let message = match serde_json::from_slice::<State>(&message.payload) {
                Ok(message) => message,
                Err(err) => {
                    warn!(id = Device::get_id(self), "Failed to parse message: {err}");
                    return;
                }
            };

            if let Some(action) = message.action {
                debug!(
                    id = Device::get_id(self),
                    ?message.action,
                    "Action received",
                );

                match action {
                    Action::LeftPressRelease => self.config.left_callback.call(self.clone()).await,
                    Action::RightPressRelease => {
                        self.config.right_callback.call(self.clone()).await
                    }
                    Action::LeftHold => self.config.left_hold_callback.call(self.clone()).await,
                    Action::RightHold => self.config.right_hold_callback.call(self.clone()).await,
                    // If there is no hold action, the switch will act like a normal release
                    Action::RightHoldRelease => {
                        if self.config.right_hold_callback.is_empty() {
                            self.config.right_callback.call(self.clone()).await
                        }
                    }
                    Action::LeftHoldRelease => {
                        if self.config.left_hold_callback.is_empty() {
                            self.config.left_callback.call(self.clone()).await
                        }
                    }
                    _ => {}
                }
            }

            if let Some(battery) = message.battery {
                self.config
                    .battery_callback
                    .call((self.clone(), battery))
                    .await;
            }
        }
    }
}
