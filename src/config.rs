use std::{
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
    devices::{
        AudioSetup, ContactSensor, DebugBridgeConfig, Device, HueBridgeConfig, IkeaOutlet,
        KasaOutlet, LightSensorConfig, PresenceConfig, WakeOnLAN,
    },
    error::{ConfigParseError, CreateDeviceError, MissingEnv},
    event::EventChannel,
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
    pub light_sensor: LightSensorConfig,
    pub hue_bridge: Option<HueBridgeConfig>,
    pub debug_bridge: Option<DebugBridgeConfig>,
    #[serde(default, with = "tuple_vec_map")]
    pub devices: Vec<(String, DeviceConfig)>,
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
pub enum DeviceConfig {
    AudioSetup(<AudioSetup as CreateDevice>::Config),
    ContactSensor(<ContactSensor as CreateDevice>::Config),
    IkeaOutlet(<IkeaOutlet as CreateDevice>::Config),
    KasaOutlet(<KasaOutlet as CreateDevice>::Config),
    WakeOnLAN(<WakeOnLAN as CreateDevice>::Config),
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

pub trait CreateDevice {
    type Config;

    fn create(
        identifier: &str,
        config: Self::Config,
        event_channel: &EventChannel,
        client: &AsyncClient,
        // TODO: Not a big fan of passing in the global config
        presence_topic: &str,
    ) -> Result<Self, CreateDeviceError>
    where
        Self: Sized;
}

macro_rules! create {
	(($self:ident, $id:ident, $event_channel:ident, $client:ident, $presence_topic:ident), [ $( $Variant:ident ),* ]) => {
		match $self {
			$(DeviceConfig::$Variant(c) => Box::new($Variant::create($id, c, $event_channel, $client, $presence_topic)?),)*
		}
    };
}

impl DeviceConfig {
    pub fn create(
        self,
        id: &str,
        event_channel: &EventChannel,
        client: &AsyncClient,
        presence_topic: &str,
    ) -> Result<Box<dyn Device>, CreateDeviceError> {
        Ok(create! {
            (self, id, event_channel, client, presence_topic), [
                AudioSetup,
                ContactSensor,
                IkeaOutlet,
                KasaOutlet,
                WakeOnLAN
            ]
        })
    }
}
