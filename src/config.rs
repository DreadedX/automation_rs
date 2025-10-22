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

#[derive(Debug)]
pub struct SetupFunction(mlua::Function);

impl Typed for SetupFunction {
    fn type_name() -> String {
        format!(
            "fun(mqtt_client: {}): {} | DeviceInterface[] | nil",
            WrappedAsyncClient::type_name(),
            Module::type_name()
        )
    }
}

impl FromLua for SetupFunction {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        Ok(Self(FromLua::from_lua(value, lua)?))
    }
}

#[derive(Debug, Default)]
pub struct Module {
    pub setup: Option<SetupFunction>,
    pub devices: Vec<Box<dyn Device>>,
    pub modules: Vec<Module>,
}

// TODO: Add option to typed to rename field
impl Typed for Module {
    fn type_name() -> String {
        "Module".into()
    }

    fn generate_header() -> Option<String> {
        Some(format!("---@class {}\n", Self::type_name()))
    }

    fn generate_members() -> Option<String> {
        Some(format!(
            r#"---@field setup {}
---@field devices {}?
---@field [number] {}?
"#,
            Option::<SetupFunction>::type_name(),
            Vec::<Box<dyn Device>>::type_name(),
            Vec::<Module>::type_name(),
        ))
    }

    fn generate_footer() -> Option<String> {
        let type_name = <Self as Typed>::type_name();
        Some(format!("local {type_name}\n"))
    }
}

impl FromLua for Module {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        // When calling require it might return a result from the searcher indicating how the
        // module was found, we want to ignore these entries.
        // TODO: Find a better solution for this
        if value.is_string() {
            return Ok(Default::default());
        }

        let mlua::Value::Table(table) = value else {
            return Err(mlua::Error::runtime(format!(
                "Expected module table, instead found: {}",
                value.type_name()
            )));
        };

        let setup = table.get("setup")?;

        let devices = table.get("devices").unwrap_or_default();
        let mut modules = Vec::new();

        for module in table.sequence_values::<Module>() {
            modules.push(module?);
        }

        Ok(Module {
            setup,
            devices,
            modules,
        })
    }
}

#[derive(Debug, Default)]
pub struct Modules(Vec<Module>);

impl Typed for Modules {
    fn type_name() -> String {
        Vec::<Module>::type_name()
    }
}

impl FromLua for Modules {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        Ok(Self(FromLua::from_lua(value, lua)?))
    }
}

impl Modules {
    pub async fn resolve(
        self,
        lua: &mlua::Lua,
        client: &WrappedAsyncClient,
    ) -> mlua::Result<Resolved> {
        let mut modules: VecDeque<_> = self.0.into();

        let mut devices = Vec::new();

        loop {
            let Some(module) = modules.pop_front() else {
                break;
            };

            modules.extend(module.modules);

            if let Some(setup) = module.setup {
                let result: mlua::Value = setup.0.call_async(client.clone()).await?;

                if result.is_nil() {
                    // We ignore nil results
                } else if let Ok(d) = <Vec<_> as FromLua>::from_lua(result.clone(), lua)
                    && !d.is_empty()
                {
                    // This is a shortcut for the common pattern of setup functions that only
                    // return devices
                    devices.extend(d);
                } else if let Ok(module) = FromLua::from_lua(result.clone(), lua) {
                    modules.push_back(module);
                } else {
                    return Err(mlua::Error::runtime(
                        "Setup function returned data in an unexpected format",
                    ));
                }
            }

            devices.extend(module.devices);
        }

        Ok(Resolved { devices })
    }
}

#[derive(Debug, Default)]
pub struct Resolved {
    pub devices: Vec<Box<dyn Device>>,
}

#[derive(Debug, LuaDeviceConfig, Typed)]
pub struct Config {
    pub fulfillment: FulfillmentConfig,
    #[device_config(from_lua, default)]
    pub modules: Modules,
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
