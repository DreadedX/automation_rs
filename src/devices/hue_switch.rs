use automation_macro::LuaDeviceConfig;
use axum::async_trait;
use rumqttc::{matches, Publish};
use tracing::{debug, trace, warn};
use zigbee2mqtt_types::vendors::philips::Zigbee929003017102;

use super::LuaDeviceCreate;
use crate::action_callback::ActionCallback;
use crate::config::{InfoConfig, MqttDeviceConfig};
use crate::devices::Device;
use crate::event::OnMqtt;
use crate::mqtt::WrappedAsyncClient;

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct Config {
    #[device_config(flatten)]
    pub info: InfoConfig,

    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,

    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,

    // TODO: IntoLua is not implemented for unit type ()
    #[device_config(from_lua, default)]
    pub left_callback: ActionCallback<bool>,

    #[device_config(from_lua, default)]
    pub right_callback: ActionCallback<bool>,
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
        // Check if the message is from the deviec itself or from a remote
        debug!(id = Device::get_id(self), "Mqtt message received");
        if matches(&message.topic, &self.config.mqtt.topic) {
            let action = match serde_json::from_slice::<Zigbee929003017102>(&message.payload) {
                Ok(message) => message.action,
                Err(err) => {
                    warn!(id = Device::get_id(self), "Failed to parse message: {err}");
                    return;
                }
            };
            debug!(id = Device::get_id(self), "Remote action = {:?}", action);

            match action {
                zigbee2mqtt_types::vendors::philips::Zigbee929003017102Action::Leftpress => {
                    self.config.left_callback.call(true).await
                }
                zigbee2mqtt_types::vendors::philips::Zigbee929003017102Action::Rightpress => {
                    self.config.right_callback.call(true).await
                }
                _ => {}
            }
        }
    }
}
