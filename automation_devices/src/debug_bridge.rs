use std::convert::Infallible;

use async_trait::async_trait;
use automation_lib::config::MqttDeviceConfig;
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_lib::mqtt::WrappedAsyncClient;
use automation_macro::{LuaDevice, LuaDeviceConfig};
use tracing::trace;

#[derive(Debug, LuaDeviceConfig, Clone)]
pub struct Config {
    pub identifier: String,
    #[device_config(flatten)]
    pub mqtt: MqttDeviceConfig,
    #[device_config(from_lua)]
    pub client: WrappedAsyncClient,
}

#[derive(Debug, Clone, LuaDevice)]
pub struct DebugBridge {
    config: Config,
}

#[async_trait]
impl LuaDeviceCreate for DebugBridge {
    type Config = Config;
    type Error = Infallible;

    async fn create(config: Self::Config) -> Result<Self, Self::Error> {
        trace!(id = config.identifier, "Setting up DebugBridge");
        Ok(Self { config })
    }
}

impl Device for DebugBridge {
    fn get_id(&self) -> String {
        self.config.identifier.clone()
    }
}
