use std::collections::{HashMap, VecDeque};
use std::net::{Ipv4Addr, SocketAddr};

use automation_lib::action_callback::ActionCallback;
use automation_lib::device::Device;
use automation_lib::mqtt::WrappedAsyncClient;
use automation_macro::LuaDeviceConfig;
use lua_typed::Typed;
use mlua::FromLua;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Setup {
    #[serde(default = "default_entrypoint")]
    pub entrypoint: String,
    #[serde(default)]
    pub variables: HashMap<String, String>,
    #[serde(default)]
    pub secrets: HashMap<String, String>,
}

fn default_entrypoint() -> String {
    "./config.lua".into()
}

#[derive(Debug, Deserialize, Typed)]
pub struct FulfillmentConfig {
    pub openid_url: String,
    #[serde(default = "default_fulfillment_ip")]
    #[typed(default)]
    pub ip: Ipv4Addr,
    #[serde(default = "default_fulfillment_port")]
    #[typed(default)]
    pub port: u16,
}

#[derive(Debug, Default)]
pub struct Devices(mlua::Value);

impl Devices {
    pub async fn get(
        self,
        lua: &mlua::Lua,
        client: &WrappedAsyncClient,
    ) -> mlua::Result<Vec<Box<dyn Device>>> {
        let mut devices = Vec::new();
        let initial_table = match self.0 {
            mlua::Value::Table(table) => table,
            mlua::Value::Function(f) => f.call_async(client.clone()).await?,
            _ => Err(mlua::Error::runtime(format!(
                "Expected table or function, instead found: {}",
                self.0.type_name()
            )))?,
        };

        let mut queue: VecDeque<mlua::Table> = [initial_table].into();
        loop {
            let Some(table) = queue.pop_front() else {
                break;
            };

            for pair in table.pairs() {
                let (_, value): (mlua::Value, _) = pair?;

                match value {
                    mlua::Value::UserData(_) => devices.push(Box::from_lua(value, lua)?),
                    mlua::Value::Function(f) => {
                        queue.push_back(f.call_async(client.clone()).await?);
                    }
                    _ => Err(mlua::Error::runtime(format!(
                        "Expected a device, table, or function, instead found: {}",
                        value.type_name()
                    )))?,
                }
            }
        }

        Ok(devices)
    }
}

impl FromLua for Devices {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        Ok(Devices(value))
    }
}

impl Typed for Devices {
    fn type_name() -> String {
        "Devices".into()
    }

    fn generate_header() -> Option<String> {
        Some(format!(
            "---@alias {} (DeviceInterface | fun(client: {}): Devices)[]\n",
            <Self as Typed>::type_name(),
            <WrappedAsyncClient as Typed>::type_name()
        ))
    }
}

#[derive(Debug, LuaDeviceConfig, Typed)]
pub struct Config {
    pub fulfillment: FulfillmentConfig,
    #[device_config(from_lua, default)]
    pub devices: Option<Devices>,
    #[device_config(from_lua)]
    pub mqtt: WrappedAsyncClient,
    #[device_config(from_lua, default)]
    #[typed(default)]
    pub schedule: HashMap<String, ActionCallback<()>>,
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
