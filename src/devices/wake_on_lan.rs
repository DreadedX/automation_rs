use std::net::Ipv4Addr;

use async_trait::async_trait;
use automation_macro::{LuaDevice, LuaDeviceConfig};
use eui48::MacAddress;
use google_home::errors::ErrorCode;
use google_home::traits::{self, Scene};
use google_home::types::Type;
use google_home::{device, GoogleHomeDevice};
use rumqttc::Publish;
use tracing::{debug, error, trace};

use super::{Device, LuaDeviceCreate};
use crate::config::{InfoConfig, MqttDeviceConfig};
use crate::event::OnMqtt;
use crate::messages::ActivateMessage;
use crate::mqtt::WrappedAsyncClient;

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct WakeOnLANConfig {
    #[device_config(flatten)]
    info: InfoConfig,
    #[device_config(flatten)]
    mqtt: MqttDeviceConfig,
    mac_address: MacAddress,
    #[device_config(default(Ipv4Addr::new(255, 255, 255, 255)))]
    broadcast_ip: Ipv4Addr,
    #[device_config(from_lua)]
    client: WrappedAsyncClient,
}

#[derive(Debug, LuaDevice)]
pub struct WakeOnLAN {
    config: WakeOnLANConfig,
}

#[async_trait]
impl LuaDeviceCreate for WakeOnLAN {
    type Config = WakeOnLANConfig;
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
    async fn on_mqtt(&mut self, message: Publish) {
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

impl GoogleHomeDevice for WakeOnLAN {
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
    async fn set_active(&self, activate: bool) -> Result<(), ErrorCode> {
        if activate {
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
        } else {
            debug!(
                id = Device::get_id(self),
                "Trying to deactivate computer, this is not currently supported"
            );
            // We do not support deactivating this scene
            Err(ErrorCode::DeviceError(
                google_home::errors::DeviceError::ActionNotAvailable,
            ))
        }
    }
}
