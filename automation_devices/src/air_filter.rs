use async_trait::async_trait;
use automation_lib::config::InfoConfig;
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_macro::{Device, LuaDeviceConfig};
use google_home::device::Name;
use google_home::errors::ErrorCode;
use google_home::traits::{
    AvailableSpeeds, FanSpeed, HumiditySetting, OnOff, Speed, SpeedValue, TemperatureControl,
    TemperatureUnit,
};
use google_home::types::Type;
use thiserror::Error;
use tracing::{debug, trace};

#[derive(Debug, Clone, LuaDeviceConfig)]
pub struct Config {
    #[device_config(flatten)]
    pub info: InfoConfig,
    pub url: String,
}

#[derive(Debug, Clone, Device)]
#[device(traits(OnOff))]
pub struct AirFilter {
    config: Config,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Connection error")]
    ReqwestError(#[from] reqwest::Error),
}

impl From<Error> for google_home::errors::ErrorCode {
    fn from(value: Error) -> Self {
        match value {
            // Assume that if we encounter a ReqwestError the device is offline
            Error::ReqwestError(_) => {
                Self::DeviceError(google_home::errors::DeviceError::DeviceOffline)
            }
        }
    }
}

// TODO: Handle error properly
impl AirFilter {
    async fn set_fan_speed(&self, speed: air_filter_types::FanSpeed) -> Result<(), Error> {
        let message = air_filter_types::SetFanSpeed::new(speed);
        let url = format!("{}/state/fan", self.config.url);
        let client = reqwest::Client::new();
        client.put(url).json(&message).send().await?;

        Ok(())
    }

    async fn get_fan_state(&self) -> Result<air_filter_types::FanState, Error> {
        let url = format!("{}/state/fan", self.config.url);
        Ok(reqwest::get(url).await?.json().await?)
    }

    async fn get_sensor_data(&self) -> Result<air_filter_types::SensorData, Error> {
        let url = format!("{}/state/sensor", self.config.url);
        Ok(reqwest::get(url).await?.json().await?)
    }
}

#[async_trait]
impl LuaDeviceCreate for AirFilter {
    type Config = Config;
    type Error = rumqttc::ClientError;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.info.identifier(), "Setting up AirFilter");

        Ok(Self { config })
    }
}

impl Device for AirFilter {
    fn get_id(&self) -> String {
        self.config.info.identifier()
    }
}

#[async_trait]
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

    async fn is_online(&self) -> bool {
        self.get_sensor_data().await.is_ok()
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
        Ok(self.get_fan_state().await?.speed != air_filter_types::FanSpeed::Off)
    }

    async fn set_on(&self, on: bool) -> Result<(), ErrorCode> {
        debug!("Turning on air filter: {on}");

        if on {
            self.set_fan_speed(air_filter_types::FanSpeed::High).await?;
        } else {
            self.set_fan_speed(air_filter_types::FanSpeed::Off).await?;
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
        let speed = self.get_fan_state().await?.speed;
        let speed = match speed {
            air_filter_types::FanSpeed::Off => "off",
            air_filter_types::FanSpeed::Low => "low",
            air_filter_types::FanSpeed::Medium => "medium",
            air_filter_types::FanSpeed::High => "high",
        };

        Ok(speed.into())
    }

    async fn set_fan_speed(&self, fan_speed: String) -> Result<(), ErrorCode> {
        let fan_speed = fan_speed.as_str();
        let speed = if fan_speed == "off" {
            air_filter_types::FanSpeed::Off
        } else if fan_speed == "low" {
            air_filter_types::FanSpeed::Low
        } else if fan_speed == "medium" {
            air_filter_types::FanSpeed::Medium
        } else if fan_speed == "high" {
            air_filter_types::FanSpeed::High
        } else {
            return Err(google_home::errors::DeviceError::TransientError.into());
        };

        self.set_fan_speed(speed).await?;

        Ok(())
    }
}

#[async_trait]
impl HumiditySetting for AirFilter {
    fn query_only_humidity_setting(&self) -> Option<bool> {
        Some(true)
    }

    async fn humidity_ambient_percent(&self) -> Result<isize, ErrorCode> {
        Ok(self.get_sensor_data().await?.humidity().round() as isize)
    }
}

#[async_trait]
impl TemperatureControl for AirFilter {
    fn query_only_temperature_control(&self) -> Option<bool> {
        Some(true)
    }

    #[allow(non_snake_case)]
    fn temperatureUnitForUX(&self) -> TemperatureUnit {
        TemperatureUnit::Celsius
    }

    async fn temperature_ambient_celsius(&self) -> Result<f32, ErrorCode> {
        // HACK: Round to one decimal place
        Ok((10.0 * self.get_sensor_data().await?.temperature()).round() / 10.0)
    }
}
