use google_home::{GoogleHomeDevice, types::Type, device, traits::{self, Scene}, errors::{ErrorCode, DeviceError}};
use log::{debug, warn};
use rumqttc::{AsyncClient, Publish};
use serde::Deserialize;

use crate::{config::{InfoConfig, MqttDeviceConfig}, mqtt::OnMqtt};

use super::Device;

pub struct WakeOnLAN {
    identifier: String,
    info: InfoConfig,
    mqtt: MqttDeviceConfig,
    mac_address: String,
}

impl WakeOnLAN {
    pub fn new(identifier: String, info: InfoConfig, mqtt: MqttDeviceConfig, mac_address: String, client: AsyncClient) -> Self {
        let t = mqtt.topic.clone();
        // @TODO Handle potential errors here
        tokio::spawn(async move {
            client.subscribe(t, rumqttc::QoS::AtLeastOnce).await.unwrap();
        });

        Self { identifier, info, mqtt, mac_address }
    }
}

impl Device for WakeOnLAN {
    fn get_id(&self) -> String {
        self.identifier.clone()
    }
}

#[derive(Debug, Deserialize)]
struct StateMessage {
    activate: bool
}

impl TryFrom<&Publish> for StateMessage {
    type Error = anyhow::Error;

    fn try_from(message: &Publish) -> Result<Self, Self::Error> {
        serde_json::from_slice(&message.payload)
            .or(Err(anyhow::anyhow!("Invalid message payload received: {:?}", message.payload)))
    }
}

impl OnMqtt for WakeOnLAN {
    fn on_mqtt(&mut self, message: &Publish) {

        if message.topic != self.mqtt.topic {
            return;
        }

        let payload = match StateMessage::try_from(message) {
            Ok(state) => state,
            Err(err) => {
                warn!("Failed to parse message: {err}");
                return;
            }
        };

        self.set_active(payload.activate).ok();
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
            tokio::spawn(async move {
                debug!("Activating Computer: {}", mac_address);
                let req = match reqwest::get(format!("http://10.0.0.2:9000/start-pc?mac={mac_address}")).await {
                    Ok(req) => req,
                    Err(err) => {
                        warn!("Failed to call webhook: {err}");
                        return;
                    }
                };
                if req.status() != 200 {
                    warn!("Failed to call webhook: {}", req.status());
                }
            });

            Ok(())
        } else {
            debug!("Trying to deactive computer, this is not currently supported");
            // We do not support deactivating this scene
            Err(ErrorCode::DeviceError(DeviceError::ActionNotAvailable))
        }
    }
}
