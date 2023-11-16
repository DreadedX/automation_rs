use std::{
    fs,
    net::{Ipv4Addr, SocketAddr},
    time::Duration,
};

use indexmap::IndexMap;
use regex::{Captures, Regex};
use rumqttc::{MqttOptions, Transport};
use serde::{Deserialize, Deserializer};
use tracing::debug;

use crate::{
    auth::OpenIDConfig,
    device_manager::DeviceConfigs,
    devices::PresenceConfig,
    error::{ConfigParseError, MissingEnv},
    schedule::Schedule,
};

#[derive(Debug, Deserialize)]
pub struct Config {
    pub openid: OpenIDConfig,
    #[serde(deserialize_with = "deserialize_mqtt_options")]
    pub mqtt: MqttOptions,
    #[serde(default)]
    pub fullfillment: FullfillmentConfig,
    pub ntfy: Option<NtfyConfig>,
    pub presence: PresenceConfig,
    pub devices: IndexMap<String, DeviceConfigs>,
    pub schedule: Schedule,
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

impl Config {
    pub fn parse_file(filename: &str) -> Result<Self, ConfigParseError> {
        debug!("Loading config: {filename}");
        let file = fs::read_to_string(filename)?;

        // Substitute in environment variables
        let re = Regex::new(r"\$\{(.*)\}").expect("Regex should be valid");
        let mut missing = MissingEnv::new();
        let file = re.replace_all(&file, |caps: &Captures| {
            let key = caps.get(1).expect("Capture group should exist").as_str();
            debug!("Substituting '{key}' in config");
            match std::env::var(key) {
                Ok(value) => value,
                Err(_) => {
                    missing.add_missing(key);
                    "".into()
                }
            }
        });

        missing.has_missing()?;

        let config: Config = serde_yaml::from_str(&file)?;

        Ok(config)
    }
}
