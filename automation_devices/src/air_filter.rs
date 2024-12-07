use std::sync::Arc;

use async_trait::async_trait;
use automation_lib::config::{InfoConfig, MqttDeviceConfig};
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_lib::event::OnMqtt;
use automation_lib::messages::{AirFilterFanState, AirFilterState, SetAirFilterFanState};
use automation_lib::mqtt::WrappedAsyncClient;
use automation_macro::LuaDeviceConfig;
use google_home::device::Name;
use google_home::errors::ErrorCode;
use google_home::traits::{
    AvailableSpeeds, FanSpeed, HumiditySetting, OnOff, Speed, SpeedValue, TemperatureSetting,
    TemperatureUnit,
};
use google_home::types::Type;
use rumqttc::Publish;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::{debug, error, trace, warn};

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct Config {
    #[device_config(flatten)]
    pub info: InfoConfig,
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

#[derive(Debug, Clone)]
pub struct AirFilter {
    config: Config,
    state: Arc<RwLock<AirFilterState>>,
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

    async fn state(&self) -> RwLockReadGuard<AirFilterState> {
        self.state.read().await
    }

    async fn state_mut(&self) -> RwLockWriteGuard<AirFilterState> {
        self.state.write().await
    }
}

#[async_trait]
impl LuaDeviceCreate for AirFilter {
    type Config = Config;
    type Error = rumqttc::ClientError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.info.identifier(), "Setting up AirFilter");

        config
            .client
            .subscribe(&config.mqtt.topic, rumqttc::QoS::AtLeastOnce)
            .await?;

        let state = AirFilterState {
            state: AirFilterFanState::Off,
            humidity: 0.0,
            temperature: 0.0,
        };
        let state = Arc::new(RwLock::new(state));

        Ok(Self { config, state })
    }
}

impl Device for AirFilter {
    fn get_id(&self) -> String {
        self.config.info.identifier()
    }
}

#[async_trait]
impl OnMqtt for AirFilter {
    async fn on_mqtt(&self, message: Publish) {
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

        if state == *self.state().await {
            return;
        }

        debug!(id = Device::get_id(self), "Updating state to {state:?}");

        *self.state_mut().await = state;
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
        Ok(self.state().await.state != AirFilterFanState::Off)
    }

    async fn set_on(&self, on: bool) -> Result<(), ErrorCode> {
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
                    speed_values: vec![SpeedValue {
                        speed_synonym: vec!["Off".into()],
                        lang: "en".into(),
                    }],
                },
                Speed {
                    speed_name: "low".into(),
                    speed_values: vec![SpeedValue {
                        speed_synonym: vec!["Low".into()],
                        lang: "en".into(),
                    }],
                },
                Speed {
                    speed_name: "medium".into(),
                    speed_values: vec![SpeedValue {
                        speed_synonym: vec!["Medium".into()],
                        lang: "en".into(),
                    }],
                },
                Speed {
                    speed_name: "high".into(),
                    speed_values: vec![SpeedValue {
                        speed_synonym: vec!["High".into()],
                        lang: "en".into(),
                    }],
                },
            ],
            ordered: true,
        }
    }

    async fn current_fan_speed_setting(&self) -> Result<String, ErrorCode> {
        let speed = match self.state().await.state {
            AirFilterFanState::Off => "off",
            AirFilterFanState::Low => "low",
            AirFilterFanState::Medium => "medium",
            AirFilterFanState::High => "high",
        };

        Ok(speed.into())
    }

    async fn set_fan_speed(&self, fan_speed: String) -> Result<(), ErrorCode> {
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

    async fn humidity_ambient_percent(&self) -> Result<isize, ErrorCode> {
        Ok(self.state().await.humidity.round() as isize)
    }
}

#[async_trait]
impl TemperatureSetting for AirFilter {
    fn query_only_temperature_control(&self) -> Option<bool> {
        Some(true)
    }

    #[allow(non_snake_case)]
    fn temperatureUnitForUX(&self) -> TemperatureUnit {
        TemperatureUnit::Celsius
    }

    async fn temperature_ambient_celsius(&self) -> f32 {
        // HACK: Round to one decimal place
        (10.0 * self.state().await.temperature).round() / 10.0
    }
}
