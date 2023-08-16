use std::net::Ipv4Addr;

use async_trait::async_trait;
use eui48::MacAddress;
use google_home::{
    device,
    errors::ErrorCode,
    traits::{self, Scene},
    types::Type,
    GoogleHomeDevice,
};
use rumqttc::Publish;
use serde::Deserialize;
use tracing::{debug, error, trace};

use crate::{
    config::{InfoConfig, MqttDeviceConfig},
    device_manager::{ConfigExternal, DeviceConfig},
    error::DeviceConfigError,
    event::OnMqtt,
    messages::ActivateMessage,
};

use super::Device;

#[derive(Debug, Clone, Deserialize)]
pub struct WakeOnLANConfig {
    #[serde(flatten)]
    info: InfoConfig,
    #[serde(flatten)]
    mqtt: MqttDeviceConfig,
    mac_address: MacAddress,
    #[serde(default = "default_broadcast_ip")]
    broadcast_ip: Ipv4Addr,
}

fn default_broadcast_ip() -> Ipv4Addr {
    Ipv4Addr::new(255, 255, 255, 255)
}

#[async_trait]
impl DeviceConfig for WakeOnLANConfig {
    async fn create(
        self,
        identifier: &str,
        _ext: &ConfigExternal,
    ) -> Result<Box<dyn Device>, DeviceConfigError> {
        trace!(
            id = identifier,
            name = self.info.name,
            room = self.info.room,
            "Setting up WakeOnLAN"
        );

        let device = WakeOnLAN {
            identifier: identifier.into(),
            info: self.info,
            mqtt: self.mqtt,
            mac_address: self.mac_address,
            broadcast_ip: self.broadcast_ip,
        };

        Ok(Box::new(device))
    }
}

#[derive(Debug)]
struct WakeOnLAN {
    identifier: String,
    info: InfoConfig,
    mqtt: MqttDeviceConfig,
    mac_address: MacAddress,
    broadcast_ip: Ipv4Addr,
}

impl Device for WakeOnLAN {
    fn get_id(&self) -> &str {
        &self.identifier
    }
}

#[async_trait]
impl OnMqtt for WakeOnLAN {
    fn topics(&self) -> Vec<&str> {
        vec![&self.mqtt.topic]
    }

    async fn on_mqtt(&mut self, message: Publish) {
        let activate = match ActivateMessage::try_from(message) {
            Ok(message) => message.activate(),
            Err(err) => {
                error!(id = self.identifier, "Failed to parse message: {err}");
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
        let mut name = device::Name::new(&self.info.name);
        name.add_default_name("Computer");

        name
    }

    fn get_id(&self) -> &str {
        Device::get_id(self)
    }

    fn is_online(&self) -> bool {
        true
    }

    fn get_room_hint(&self) -> Option<&str> {
        self.info.room.as_deref()
    }
}

#[async_trait]
impl traits::Scene for WakeOnLAN {
    async fn set_active(&self, activate: bool) -> Result<(), ErrorCode> {
        if activate {
            debug!(
                id = self.identifier,
                "Activating Computer: {} (Sending to {})", self.mac_address, self.broadcast_ip
            );
            let wol =
                wakey::WolPacket::from_bytes(&self.mac_address.to_array()).map_err(|err| {
                    error!(id = self.identifier, "invalid mac address: {err}");
                    google_home::errors::DeviceError::TransientError
                })?;

            wol.send_magic_to((Ipv4Addr::new(0, 0, 0, 0), 0), (self.broadcast_ip, 9))
                .await
                .map_err(|err| {
                    error!(id = self.identifier, "Failed to activate computer: {err}");
                    google_home::errors::DeviceError::TransientError.into()
                })
                .map(|_| debug!(id = self.identifier, "Success!"))
        } else {
            debug!(
                id = self.identifier,
                "Trying to deactive computer, this is not currently supported"
            );
            // We do not support deactivating this scene
            Err(ErrorCode::DeviceError(
                google_home::errors::DeviceError::ActionNotAvailable,
            ))
        }
    }
}
