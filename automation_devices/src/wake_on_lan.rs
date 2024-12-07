use std::net::Ipv4Addr;

use async_trait::async_trait;
use automation_lib::config::{InfoConfig, MqttDeviceConfig};
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_lib::event::OnMqtt;
use automation_lib::messages::ActivateMessage;
use automation_lib::mqtt::WrappedAsyncClient;
use automation_macro::LuaDeviceConfig;
use eui48::MacAddress;
use google_home::device;
use google_home::errors::ErrorCode;
use google_home::traits::{self, Scene};
use google_home::types::Type;
use rumqttc::Publish;
use tracing::{debug, error, trace};

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct Config {
    #[device_config(flatten)]
    pub info: InfoConfig,
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    pub mac_address: MacAddress,
    #[device_config(default(Ipv4Addr::new(255, 255, 255, 255)))]
    pub broadcast_ip: Ipv4Addr,
    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

#[derive(Debug, Clone)]
pub struct WakeOnLAN {
    config: Config,
}

#[async_trait]
impl LuaDeviceCreate for WakeOnLAN {
    type Config = Config;
    type Error = rumqttc::ClientError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.info.identifier(), "Setting up WakeOnLAN");

        config
            .client
            .subscribe(&config.mqtt.topic, rumqttc::QoS::AtLeastOnce)
            .await?;

        Ok(Self { config })
    }
}

impl Device for WakeOnLAN {
    fn get_id(&self) -> String {
        self.config.info.identifier()
    }
}

#[async_trait]
impl OnMqtt for WakeOnLAN {
    async fn on_mqtt(&self, message: Publish) {
        if !rumqttc::matches(&message.topic, &self.config.mqtt.topic) {
            return;
        }

        let activate = match ActivateMessage::try_from(message) {
            Ok(message) => message.activate(),
            Err(err) => {
                error!(id = Device::get_id(self), "Failed to parse message: {err}");
                return;
            }
        };

        self.set_active(activate).await.ok();
    }
}

impl google_home::Device for WakeOnLAN {
    fn get_device_type(&self) -> Type {
        Type::Scene
    }

    fn get_device_name(&self) -> device::Name {
        let mut name = device::Name::new(&self.config.info.name);
        name.add_default_name("Computer");

        name
    }

    fn get_id(&self) -> String {
        Device::get_id(self)
    }

    fn is_online(&self) -> bool {
        true
    }

    fn get_room_hint(&self) -> Option<&str> {
        self.config.info.room.as_deref()
    }
}

#[async_trait]
impl traits::Scene for WakeOnLAN {
    async fn set_active(&self, deactivate: bool) -> Result<(), ErrorCode> {
        if deactivate {
            debug!(
                id = Device::get_id(self),
                "Trying to deactivate computer, this is not currently supported"
            );
            // We do not support deactivating this scene
            Err(ErrorCode::DeviceError(
                google_home::errors::DeviceError::ActionNotAvailable,
            ))
        } else {
            debug!(
                id = Device::get_id(self),
                "Activating Computer: {} (Sending to {})",
                self.config.mac_address,
                self.config.broadcast_ip
            );
            let wol = wakey::WolPacket::from_bytes(&self.config.mac_address.to_array()).map_err(
                |err| {
                    error!(id = Device::get_id(self), "invalid mac address: {err}");
                    google_home::errors::DeviceError::TransientError
                },
            )?;

            wol.send_magic_to(
                (Ipv4Addr::new(0, 0, 0, 0), 0),
                (self.config.broadcast_ip, 9),
            )
            .await
            .map_err(|err| {
                error!(
                    id = Device::get_id(self),
                    "Failed to activate computer: {err}"
                );
                google_home::errors::DeviceError::TransientError.into()
            })
            .map(|_| debug!(id = Device::get_id(self), "Success!"))
        }
    }
}
