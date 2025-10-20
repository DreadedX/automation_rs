use std::collections::{HashMap, VecDeque};
use std::net::{Ipv4Addr, SocketAddr};

use automation_lib::action_callback::ActionCallback;
use automation_lib::device::Device;
use automation_lib::mqtt::{MqttConfig, WrappedAsyncClient};
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
    "./config/config.lua".into()
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
pub struct Modules(mlua::Value);

impl Modules {
    pub async fn setup(
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
                let (name, value): (String, _) = pair?;

                match value {
                    mlua::Value::Table(table) => queue.push_back(table),
                    mlua::Value::UserData(_)
                        if let Ok(device) = Box::from_lua(value.clone(), lua) =>
                    {
                        devices.push(device);
                    }
                    mlua::Value::Function(f) if name == "setup" => {
                        let value: mlua::Value = f.call_async(client.clone()).await?;
                        if let Some(table) = value.as_table() {
                            queue.push_back(table.clone());
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(devices)
    }
}

impl FromLua for Modules {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        Ok(Modules(value))
    }
}

impl Typed for Modules {
    fn type_name() -> String {
        "Modules".into()
    }

    fn generate_header() -> Option<String> {
        let type_name = Self::type_name();
        let client_type = WrappedAsyncClient::type_name();

        Some(format!(
            r#"---@alias SetupFunction fun(mqtt_client: {client_type}): SetupTable?
---@alias SetupTable (DeviceInterface | {{ setup: SetupFunction? }} | SetupTable)[]
---@alias {type_name} SetupFunction | SetupTable
"#,
        ))
    }
}

#[derive(Debug, LuaDeviceConfig, Typed)]
pub struct Config {
    pub fulfillment: FulfillmentConfig,
    #[device_config(from_lua, default)]
    pub modules: Option<Modules>,
    #[device_config(from_lua)]
    pub mqtt: MqttConfig,
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
