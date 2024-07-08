use async_trait::async_trait;
use automation_macro::{LuaDevice, LuaDeviceConfig};
use google_home::device::Name;
use google_home::errors::ErrorCode;
use google_home::traits::{AvailableSpeeds, FanSpeed, HumiditySetting, OnOff, Speed, SpeedValues};
use google_home::types::Type;
use rumqttc::Publish;
use tracing::{debug, error, trace, warn};

use super::LuaDeviceCreate;
use crate::config::{InfoConfig, MqttDeviceConfig};
use crate::devices::Device;
use crate::event::OnMqtt;
use crate::messages::{AirFilterFanState, AirFilterState, SetAirFilterFanState};
use crate::mqtt::WrappedAsyncClient;

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct AirFilterConfig {
    #[device_config(flatten)]
    pub info: InfoConfig,
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

#[derive(Debug, LuaDevice)]
pub struct AirFilter {
    config: AirFilterConfig,

    last_known_state: AirFilterState,
}

impl AirFilter {
    async fn set_speed(&self, state: AirFilterFanState) {
        let message = SetAirFilterFanState::new(state);

        let topic = format!("{}/set", self.config.mqtt.topic);
        // TODO: Handle potential errors here
        self.config
            .client
            .publish(
                &topic,
                rumqttc::QoS::AtLeastOnce,
                false,
                serde_json::to_string(&message).unwrap(),
            )
            .await
            .map_err(|err| warn!("Failed to update state on {topic}: {err}"))
            .ok();
    }
}

#[async_trait]
impl LuaDeviceCreate for AirFilter {
    type Config = AirFilterConfig;
    type Error = rumqttc::ClientError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.info.identifier(), "Setting up AirFilter");

        config
            .client
            .subscribe(&config.mqtt.topic, rumqttc::QoS::AtLeastOnce)
            .await?;

        Ok(Self {
            config,
            last_known_state: AirFilterState {
                state: AirFilterFanState::Off,
                humidity: 0.0,
            },
        })
    }
}

impl Device for AirFilter {
    fn get_id(&self) -> String {
        self.config.info.identifier()
    }
}

#[async_trait]
impl OnMqtt for AirFilter {
    async fn on_mqtt(&mut self, message: Publish) {
        if !rumqttc::matches(&message.topic, &self.config.mqtt.topic) {
            return;
        }

        let state = match AirFilterState::try_from(message) {
            Ok(state) => state,
            Err(err) => {
                error!(id = Device::get_id(self), "Failed to parse message: {err}");
                return;
            }
        };

        if state == self.last_known_state {
            return;
        }

        debug!(id = Device::get_id(self), "Updating state to {state:?}");

        self.last_known_state = state;
    }
}

impl google_home::Device for AirFilter {
    fn get_device_type(&self) -> Type {
        Type::AirPurifier
    }

    fn get_device_name(&self) -> Name {
        Name::new(&self.config.info.name)
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

    fn will_report_state(&self) -> bool {
        false
    }
}

#[async_trait]
impl OnOff for AirFilter {
    async fn on(&self) -> Result<bool, ErrorCode> {
        Ok(self.last_known_state.state != AirFilterFanState::Off)
    }

    async fn set_on(&mut self, on: bool) -> Result<(), ErrorCode> {
        debug!("Turning on air filter: {on}");

        if on {
            self.set_speed(AirFilterFanState::High).await;
        } else {
            self.set_speed(AirFilterFanState::Off).await;
        }

        Ok(())
    }
}

#[async_trait]
impl FanSpeed for AirFilter {
    fn available_fan_speeds(&self) -> AvailableSpeeds {
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

    fn current_fan_speed_setting(&self) -> Result<String, ErrorCode> {
        let speed = match self.last_known_state.state {
            AirFilterFanState::Off => "off",
            AirFilterFanState::Low => "low",
            AirFilterFanState::Medium => "medium",
            AirFilterFanState::High => "high",
        };

        Ok(speed.into())
    }

    async fn set_fan_speed(&mut self, fan_speed: String) -> Result<(), ErrorCode> {
        let fan_speed = fan_speed.as_str();
        let state = if fan_speed == "off" {
            AirFilterFanState::Off
        } else if fan_speed == "low" {
            AirFilterFanState::Low
        } else if fan_speed == "medium" {
            AirFilterFanState::Medium
        } else if fan_speed == "high" {
            AirFilterFanState::High
        } else {
            return Err(google_home::errors::DeviceError::TransientError.into());
        };

        self.set_speed(state).await;

        Ok(())
    }
}

#[async_trait]
impl HumiditySetting for AirFilter {
    fn query_only_humidity_setting(&self) -> Option<bool> {
        Some(true)
    }

    fn humidity_ambient_percent(&self) -> Result<isize, ErrorCode> {
        Ok(self.last_known_state.humidity.round() as isize)
    }
}
