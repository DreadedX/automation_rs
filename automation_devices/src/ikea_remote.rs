use automation_lib::action_callback::ActionCallback;
use automation_lib::config::{InfoConfig, MqttDeviceConfig};
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_lib::event::OnMqtt;
use automation_lib::messages::{RemoteAction, RemoteMessage};
use automation_lib::mqtt::WrappedAsyncClient;
use automation_macro::LuaDeviceConfig;
use axum::async_trait;
use rumqttc::{matches, Publish};
use tracing::{debug, error, trace};

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct Config {
    #[device_config(flatten)]
    pub info: InfoConfig,

    #[device_config(default)]
    pub single_button: bool,

    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,

    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,

    #[device_config(from_lua)]
    pub callback: ActionCallback<IkeaRemote, bool>,
}

#[derive(Debug, Clone)]
pub struct IkeaRemote {
    config: Config,
}

impl Device for IkeaRemote {
    fn get_id(&self) -> String {
        self.config.info.identifier()
    }
}

#[async_trait]
impl LuaDeviceCreate for IkeaRemote {
    type Config = Config;
    type Error = rumqttc::ClientError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.info.identifier(), "Setting up IkeaRemote");

        config
            .client
            .subscribe(&config.mqtt.topic, rumqttc::QoS::AtLeastOnce)
            .await?;

        Ok(Self { config })
    }
}

#[async_trait]
impl OnMqtt for IkeaRemote {
    async fn on_mqtt(&self, message: Publish) {
        // Check if the message is from the deviec itself or from a remote
        if matches(&message.topic, &self.config.mqtt.topic) {
            let action = match RemoteMessage::try_from(message) {
                Ok(message) => message.action(),
                Err(err) => {
                    error!(id = Device::get_id(self), "Failed to parse message: {err}");
                    return;
                }
            };
            debug!(id = Device::get_id(self), "Remote action = {:?}", action);

            let on = if self.config.single_button {
                match action {
                    RemoteAction::On => Some(true),
                    RemoteAction::BrightnessMoveUp => Some(false),
                    _ => None,
                }
            } else {
                match action {
                    RemoteAction::On => Some(true),
                    RemoteAction::Off => Some(false),
                    _ => None,
                }
            };

            if let Some(on) = on {
                self.config.callback.call(self, &on).await;
            }
        }
    }
}
