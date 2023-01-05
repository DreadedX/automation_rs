use google_home::{GoogleHomeDevice, types::Type, device, traits::{self, Scene}, errors::{ErrorCode, DeviceError}};
use tracing::{debug, error};
use rumqttc::{AsyncClient, Publish};
use pollster::FutureExt as _;

use crate::{config::{InfoConfig, MqttDeviceConfig}, mqtt::{OnMqtt, ActivateMessage}};

use super::Device;

pub struct WakeOnLAN {
    identifier: String,
    info: InfoConfig,
    mqtt: MqttDeviceConfig,
    mac_address: String,
}

impl WakeOnLAN {
    pub fn new(identifier: String, info: InfoConfig, mqtt: MqttDeviceConfig, mac_address: String, client: AsyncClient) -> Self {
        // @TODO Handle potential errors here
        client.subscribe(mqtt.topic.clone(), rumqttc::QoS::AtLeastOnce).block_on().unwrap();

        Self { identifier, info, mqtt, mac_address }
    }
}

impl Device for WakeOnLAN {
    fn get_id(&self) -> String {
        self.identifier.clone()
    }
}

impl OnMqtt for WakeOnLAN {
    fn on_mqtt(&mut self, message: &Publish) {
        if message.topic != self.mqtt.topic {
            return;
        }

        let activate = match ActivateMessage::try_from(message) {
            Ok(message) => message.activate(),
            Err(err) => {
                error!(id = self.identifier, "Failed to parse message: {err}");
                return;
            }
        };

        self.set_active(activate).ok();
    }
}

impl GoogleHomeDevice for WakeOnLAN {
    fn get_device_type(&self) -> Type {
        Type::Scene
    }

    fn get_device_name(&self) -> device::Name {
        let mut name = device::Name::new(&self.info.name);
        name.add_default_name("Computer");

        return name;
    }

    fn get_id(&self) -> String {
        Device::get_id(self)
    }

    fn is_online(&self) -> bool {
        true
    }

    fn get_room_hint(&self) -> Option<String> {
        self.info.room.clone()
    }
}

impl traits::Scene for WakeOnLAN {
    fn set_active(&self, activate: bool) -> Result<(), ErrorCode> {
        if activate {
            // @TODO In the future send the wake on lan package directly, this is kind of annoying
            // if we are inside of docker, so for now just call a webhook that does it for us
            let mac_address = self.mac_address.clone();
            let id = self.identifier.clone();

            debug!(id, "Activating Computer: {}", mac_address);
            let res = match reqwest::get(format!("http://10.0.0.2:9000/start-pc?mac={mac_address}")).block_on() {
                Ok(res) => res,
                Err(err) => {
                    error!(id, "Failed to call webhook: {err}");
                    return Err(DeviceError::TransientError.into());
                }
            };

            let status = res.status();
            if !status.is_success() {
                error!(id, "Failed to call webhook: {}", status);
            }

            Ok(())
        } else {
            debug!(id = self.identifier, "Trying to deactive computer, this is not currently supported");
            // We do not support deactivating this scene
            Err(ErrorCode::DeviceError(DeviceError::ActionNotAvailable))
        }
    }
}
