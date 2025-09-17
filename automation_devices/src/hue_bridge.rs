use std::convert::Infallible;
use std::net::SocketAddr;

use async_trait::async_trait;
use automation_lib::device::{Device, LuaDeviceCreate};
use automation_macro::{Device, LuaDeviceConfig};
use lua_typed::Typed;
use mlua::LuaSerdeExt;
use serde::{Deserialize, Serialize};
use tracing::{error, trace, warn};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Flag {
    Presence,
    Darkness,
}

#[derive(Debug, Clone, Deserialize, Typed)]
pub struct FlagIDs {
    presence: isize,
    darkness: isize,
}
crate::register_type!(FlagIDs);

#[derive(Debug, LuaDeviceConfig, Clone, Typed)]
#[typed(as = "HueBridgeConfig")]
pub struct Config {
    pub identifier: String,
    #[device_config(rename("ip"), with(|ip| SocketAddr::new(ip, 80)))]
    #[typed(as = "ip")]
    pub addr: SocketAddr,
    pub login: String,
    pub flags: FlagIDs,
}
crate::register_type!(Config);

#[derive(Debug, Clone, Device)]
#[device(add_methods(Self::add_methods))]
pub struct HueBridge {
    config: Config,
}
crate::register_device!(HueBridge);

#[derive(Debug, Serialize)]
struct FlagMessage {
    flag: bool,
}

#[async_trait]
impl LuaDeviceCreate for HueBridge {
    type Config = Config;
    type Error = Infallible;

    async fn create(config: Self::Config) -> Result<Self, Infallible> {
        trace!(id = config.identifier, "Setting up HueBridge");
        Ok(Self { config })
    }
}

impl HueBridge {
    pub async fn set_flag(&self, flag: Flag, value: bool) {
        let flag_id = match flag {
            Flag::Presence => self.config.flags.presence,
            Flag::Darkness => self.config.flags.darkness,
        };

        let url = format!(
            "http://{}/api/{}/sensors/{flag_id}/state",
            self.config.addr, self.config.login
        );

        trace!(?flag, flag_id, value, "Sending request to change flag");
        let res = reqwest::Client::new()
            .put(url)
            .json(&FlagMessage { flag: value })
            .send()
            .await;

        match res {
            Ok(res) => {
                let status = res.status();
                if !status.is_success() {
                    warn!(flag_id, "Status code is not success: {status}");
                }
            }
            Err(err) => {
                error!(flag_id, "Error: {err}");
            }
        }
    }

    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method(
            "set_flag",
            async |lua, this, (flag, value): (mlua::Value, bool)| {
                let flag: Flag = lua.from_value(flag)?;

                this.set_flag(flag, value).await;

                Ok(())
            },
        );
    }
}

impl Device for HueBridge {
    fn get_id(&self) -> String {
        self.config.identifier.clone()
    }
}
