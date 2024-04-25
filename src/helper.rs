use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use mlua::FromLua;
use serde::Deserialize;

use crate::event::{self, EventChannel};

#[derive(Debug, Deserialize)]
pub struct DurationSeconds(u64);

impl From<DurationSeconds> for Duration {
    fn from(value: DurationSeconds) -> Self {
        Self::from_secs(value.0)
    }
}

#[derive(Debug, Deserialize)]
pub struct Ipv4SocketAddr<const PORT: u16>(Ipv4Addr);

impl<const PORT: u16> From<Ipv4SocketAddr<PORT>> for SocketAddr {
    fn from(ip: Ipv4SocketAddr<PORT>) -> Self {
        Self::from((ip.0, PORT))
    }
}

#[derive(Debug, Clone)]
pub struct TxHelper(EventChannel);

impl<'lua> FromLua<'lua> for TxHelper {
    fn from_lua(value: mlua::Value<'lua>, lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        Ok(TxHelper(mlua::FromLua::from_lua(value, lua)?))
    }
}

impl From<TxHelper> for event::Sender {
    fn from(value: TxHelper) -> Self {
        value.0.get_tx()
    }
}
