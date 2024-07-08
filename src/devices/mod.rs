mod air_filter;
mod audio_setup;
mod contact_sensor;
mod debug_bridge;
mod hue_bridge;
mod hue_group;
mod ikea_outlet;
mod kasa_outlet;
mod light_sensor;
mod ntfy;
mod presence;
mod wake_on_lan;
mod washer;

use std::fmt::Debug;

use async_trait::async_trait;
use automation_cast::Cast;
use google_home::traits::OnOff;

pub use self::air_filter::*;
pub use self::audio_setup::*;
pub use self::contact_sensor::*;
pub use self::debug_bridge::*;
pub use self::hue_bridge::*;
pub use self::hue_group::*;
pub use self::ikea_outlet::*;
pub use self::kasa_outlet::*;
pub use self::light_sensor::*;
pub use self::ntfy::{Notification, Ntfy};
pub use self::presence::{Presence, PresenceConfig, DEFAULT_PRESENCE};
pub use self::wake_on_lan::*;
pub use self::washer::*;
use crate::event::{OnDarkness, OnMqtt, OnNotification, OnPresence};
use crate::traits::Timeout;

#[async_trait]
pub trait LuaDeviceCreate {
    type Config;
    type Error;

    async fn create(config: Self::Config) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

pub fn register_with_lua(lua: &mlua::Lua) -> mlua::Result<()> {
    AirFilter::register_with_lua(lua)?;
    AudioSetup::register_with_lua(lua)?;
    ContactSensor::register_with_lua(lua)?;
    DebugBridge::register_with_lua(lua)?;
    HueBridge::register_with_lua(lua)?;
    HueGroup::register_with_lua(lua)?;
    IkeaOutlet::register_with_lua(lua)?;
    KasaOutlet::register_with_lua(lua)?;
    LightSensor::register_with_lua(lua)?;
    Ntfy::register_with_lua(lua)?;
    Presence::register_with_lua(lua)?;
    WakeOnLAN::register_with_lua(lua)?;
    Washer::register_with_lua(lua)?;

    Ok(())
}

pub trait Device:
    Debug
    + Sync
    + Send
    + Cast<dyn google_home::Device>
    + Cast<dyn OnMqtt>
    + Cast<dyn OnMqtt>
    + Cast<dyn OnPresence>
    + Cast<dyn OnDarkness>
    + Cast<dyn OnNotification>
    + Cast<dyn OnOff>
    + Cast<dyn Timeout>
{
    fn get_id(&self) -> String;
}
