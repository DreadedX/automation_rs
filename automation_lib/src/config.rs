use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use rumqttc::{MqttOptions, Transport};
use serde::Deserialize;

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

#[derive(Debug, Deserialize)]
pub struct FulfillmentConfig {
    pub openid_url: String,
    #[serde(default = "default_fulfillment_ip")]
    pub ip: Ipv4Addr,
    #[serde(default = "default_fulfillment_port")]
    pub port: u16,
}

impl From<FulfillmentConfig> for SocketAddr {
    fn from(fulfillment: FulfillmentConfig) -> Self {
        (fulfillment.ip, fulfillment.port).into()
    }
}

fn default_fulfillment_ip() -> Ipv4Addr {
    [0, 0, 0, 0].into()
}

fn default_fulfillment_port() -> u16 {
    7878
}

#[derive(Debug, Clone, Deserialize)]
pub struct InfoConfig {
    pub name: String,
    pub room: Option<String>,
}

impl InfoConfig {
    pub fn identifier(&self) -> String {
        (if let Some(room) = &self.room {
            room.to_ascii_lowercase().replace(' ', "_") + "_"
        } else {
            String::new()
        }) + &self.name.to_ascii_lowercase().replace(' ', "_")
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MqttDeviceConfig {
    pub topic: String,
}
