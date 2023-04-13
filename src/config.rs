use std::{
    collections::HashMap,
    fs,
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};

use regex::{Captures, Regex};
use rumqttc::{AsyncClient, MqttOptions, Transport};
use serde::{Deserialize, Deserializer};
use tracing::debug;

use crate::{
    auth::OpenIDConfig,
    debug_bridge::DebugBridgeConfig,
    devices::{
        self, AudioSetup, AudioSetupConfig, ContactSensor, ContactSensorConfig, IkeaOutlet,
        IkeaOutletConfig, KasaOutlet, KasaOutletConfig, WakeOnLAN, WakeOnLANConfig,
    },
    error::{ConfigParseError, DeviceCreateError, MissingEnv},
    hue_bridge::HueBridgeConfig,
    light_sensor::LightSensorConfig,
};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub openid: OpenIDConfig,
    #[serde(deserialize_with = "deserialize_mqtt_options")]
    pub mqtt: MqttOptions,
    #[serde(default)]
    pub fullfillment: FullfillmentConfig,
    pub ntfy: Option<NtfyConfig>,
    pub presence: MqttDeviceConfig,
    pub light_sensor: LightSensorConfig,
    pub hue_bridge: Option<HueBridgeConfig>,
    pub debug_bridge: Option<DebugBridgeConfig>,
    #[serde(default)]
    pub devices: HashMap<String, Device>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub client_name: String,
    pub username: String,
    pub password: String,
    #[serde(default)]
    pub tls: bool,
}

impl From<MqttConfig> for MqttOptions {
    fn from(value: MqttConfig) -> Self {
        let mut mqtt_options = MqttOptions::new(value.client_name, value.host, value.port);
        mqtt_options.set_credentials(value.username, value.password);
        mqtt_options.set_keep_alive(Duration::from_secs(5));

        if value.tls {
            mqtt_options.set_transport(Transport::tls_with_default_config());
        }

        mqtt_options
    }
}

fn deserialize_mqtt_options<'de, D>(deserializer: D) -> Result<MqttOptions, D::Error>
where
    D: Deserializer<'de>,
{
    Ok(MqttOptions::from(MqttConfig::deserialize(deserializer)?))
}

#[derive(Debug, Deserialize)]
pub struct FullfillmentConfig {
    #[serde(default = "default_fullfillment_ip")]
    pub ip: Ipv4Addr,
    #[serde(default = "default_fullfillment_port")]
    pub port: u16,
}

impl From<FullfillmentConfig> for SocketAddr {
    fn from(fullfillment: FullfillmentConfig) -> Self {
        (fullfillment.ip, fullfillment.port).into()
    }
}

impl Default for FullfillmentConfig {
    fn default() -> Self {
        Self {
            ip: default_fullfillment_ip(),
            port: default_fullfillment_port(),
        }
    }
}

fn default_fullfillment_ip() -> Ipv4Addr {
    [0, 0, 0, 0].into()
}

fn default_fullfillment_port() -> u16 {
    7878
}

#[derive(Debug, Deserialize)]
pub struct NtfyConfig {
    #[serde(default = "default_ntfy_url")]
    pub url: String,
    pub topic: String,
}

fn default_ntfy_url() -> String {
    "https://ntfy.sh".into()
}

#[derive(Debug, Clone, Deserialize)]
pub struct InfoConfig {
    pub name: String,
    pub room: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MqttDeviceConfig {
    pub topic: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Device {
    IkeaOutlet(IkeaOutletConfig),
    WakeOnLAN(WakeOnLANConfig),
    KasaOutlet(KasaOutletConfig),
    AudioSetup(AudioSetupConfig),
    ContactSensor(ContactSensorConfig),
}

impl Config {
    pub fn parse_file(filename: &str) -> Result<Self, ConfigParseError> {
        debug!("Loading config: {filename}");
        let file = fs::read_to_string(filename)?;

        // Substitute in environment variables
        let re = Regex::new(r"\$\{(.*)\}").unwrap();
        let mut missing = MissingEnv::new();
        let file = re.replace_all(&file, |caps: &Captures| {
            let key = caps.get(1).unwrap().as_str();
            debug!("Substituting '{key}' in config");
            match std::env::var(key) {
                Ok(value) => value,
                Err(_) => {
                    missing.add_missing(key);
                    "".to_string()
                }
            }
        });

        missing.has_missing()?;

        let config: Config = toml::from_str(&file)?;

        Ok(config)
    }
}

impl Device {
    pub fn create(
        self,
        identifier: &str,
        client: AsyncClient,
        presence_topic: &str,
    ) -> Result<Box<dyn devices::Device>, DeviceCreateError> {
        let device: Box<dyn devices::Device> = match self {
            Device::IkeaOutlet(c) => Box::new(IkeaOutlet::create(identifier, c, client)?),
            Device::WakeOnLAN(c) => Box::new(WakeOnLAN::create(identifier, c)?),
            Device::KasaOutlet(c) => Box::new(KasaOutlet::create(identifier, c)?),
            Device::AudioSetup(c) => {
                Box::new(AudioSetup::create(identifier, c, client, presence_topic)?)
            }
            Device::ContactSensor(c) => Box::new(ContactSensor::create(
                identifier,
                c,
                client,
                presence_topic,
            )?),
        };

        Ok(device)
    }
}
