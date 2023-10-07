use async_trait::async_trait;
use google_home::device::Name;
use google_home::errors::ErrorCode;
use google_home::traits::{AvailableSpeeds, FanSpeed, OnOff, Speed, SpeedValues};
use google_home::types::Type;
use google_home::GoogleHomeDevice;
use rumqttc::{AsyncClient, Publish};
use serde::Deserialize;
use tracing::{debug, error, warn};

use crate::config::{InfoConfig, MqttDeviceConfig};
use crate::device_manager::{ConfigExternal, DeviceConfig};
use crate::devices::Device;
use crate::error::DeviceConfigError;
use crate::event::OnMqtt;
use crate::messages::{AirFilterMessage, AirFilterState};

#[derive(Debug, Deserialize)]
pub struct AirFilterConfig {
    #[serde(flatten)]
    info: InfoConfig,
    #[serde(flatten)]
    mqtt: MqttDeviceConfig,
}

#[async_trait]
impl DeviceConfig for AirFilterConfig {
    async fn create(
        self,
        identifier: &str,
        ext: &ConfigExternal,
    ) -> Result<Box<dyn Device>, DeviceConfigError> {
        let device = AirFilter {
            identifier: identifier.into(),
            info: self.info,
            mqtt: self.mqtt,
            client: ext.client.clone(),
            last_known_state: AirFilterState::Off,
        };

        Ok(Box::new(device))
    }
}

#[derive(Debug)]
pub struct AirFilter {
    identifier: String,
    info: InfoConfig,
    mqtt: MqttDeviceConfig,

    client: AsyncClient,
    last_known_state: AirFilterState,
}

impl AirFilter {
    async fn set_speed(&self, state: AirFilterState) {
        let message = AirFilterMessage::new(state);

        let topic = format!("{}/set", self.mqtt.topic);
        // TODO: Handle potential errors here
        self.client
            .publish(
                topic.clone(),
                rumqttc::QoS::AtLeastOnce,
                false,
                serde_json::to_string(&message).unwrap(),
            )
            .await
            .map_err(|err| warn!("Failed to update state on {topic}: {err}"))
            .ok();
    }
}

impl Device for AirFilter {
    fn get_id(&self) -> &str {
        &self.identifier
    }
}

#[async_trait]
impl OnMqtt for AirFilter {
    fn topics(&self) -> Vec<&str> {
        vec![&self.mqtt.topic]
    }

    async fn on_mqtt(&mut self, message: Publish) {
        let state = match AirFilterMessage::try_from(message) {
            Ok(state) => state.state(),
            Err(err) => {
                error!(id = self.identifier, "Failed to parse message: {err}");
                return;
            }
        };

        if state == self.last_known_state {
            return;
        }

        debug!(id = self.identifier, "Updating state to {state:?}");

        self.last_known_state = state;
    }
}

impl GoogleHomeDevice for AirFilter {
    fn get_device_type(&self) -> Type {
        Type::AirPurifier
    }

    fn get_device_name(&self) -> Name {
        Name::new(&self.info.name)
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

    fn will_report_state(&self) -> bool {
        false
    }
}

#[async_trait]
impl OnOff for AirFilter {
    async fn is_on(&self) -> Result<bool, ErrorCode> {
        Ok(self.last_known_state != AirFilterState::Off)
    }

    async fn set_on(&mut self, on: bool) -> Result<(), ErrorCode> {
        debug!("Turning on air filter: {on}");

        if on {
            self.set_speed(AirFilterState::High).await;
        } else {
            self.set_speed(AirFilterState::Off).await;
        }

        Ok(())
    }
}

#[async_trait]
impl FanSpeed for AirFilter {
    fn available_speeds(&self) -> AvailableSpeeds {
        AvailableSpeeds {
            speeds: vec![
                Speed {
                    speed_name: "off".into(),
                    speed_values: vec![SpeedValues {
                        speed_synonym: vec!["Off".into()],
                        lang: "en".into(),
                    }],
                },
                Speed {
                    speed_name: "low".into(),
                    speed_values: vec![SpeedValues {
                        speed_synonym: vec!["Low".into()],
                        lang: "en".into(),
                    }],
                },
                Speed {
                    speed_name: "medium".into(),
                    speed_values: vec![SpeedValues {
                        speed_synonym: vec!["Medium".into()],
                        lang: "en".into(),
                    }],
                },
                Speed {
                    speed_name: "high".into(),
                    speed_values: vec![SpeedValues {
                        speed_synonym: vec!["High".into()],
                        lang: "en".into(),
                    }],
                },
            ],
            ordered: true,
        }
    }

    async fn current_speed(&self) -> String {
        let speed = match self.last_known_state {
            AirFilterState::Off => "off",
            AirFilterState::Low => "low",
            AirFilterState::Medium => "medium",
            AirFilterState::High => "high",
        };

        speed.into()
    }

    async fn set_speed(&self, speed: &str) -> Result<(), ErrorCode> {
        let state = if speed == "off" {
            AirFilterState::Off
        } else if speed == "low" {
            AirFilterState::Low
        } else if speed == "medium" {
            AirFilterState::Medium
        } else if speed == "high" {
            AirFilterState::High
        } else {
            return Err(google_home::errors::DeviceError::TransientError.into());
        };

        self.set_speed(state).await;

        Ok(())
    }
}
