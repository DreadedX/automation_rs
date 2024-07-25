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

pub use self::air_filter::AirFilter;
pub use self::audio_setup::AudioSetup;
pub use self::contact_sensor::ContactSensor;
pub use self::debug_bridge::DebugBridge;
pub use self::hue_bridge::HueBridge;
pub use self::hue_group::HueGroup;
pub use self::ikea_outlet::IkeaOutlet;
pub use self::kasa_outlet::KasaOutlet;
pub use self::light_sensor::LightSensor;
pub use self::ntfy::{Notification, Ntfy};
pub use self::presence::{Presence, DEFAULT_PRESENCE};
pub use self::wake_on_lan::WakeOnLAN;
pub use self::washer::Washer;
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

macro_rules! register_device {
    ($lua:expr, $device:ty) => {
        $lua.globals()
            .set(stringify!($device), $lua.create_proxy::<$device>()?)?;
    };
}

macro_rules! impl_device {
    ($lua:expr, $device:ty) => {
        impl mlua::UserData for $device {
            fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
                methods.add_async_function("new", |lua, config: mlua::Value| async {
                    let config = mlua::FromLua::from_lua(config, lua)?;

                    // TODO: Using crate:: could cause issues
                    let device: $device = crate::devices::LuaDeviceCreate::create(config)
                        .await
                        .map_err(mlua::ExternalError::into_lua_err)?;

                    Ok(crate::device_manager::WrappedDevice::new(Box::new(device)))
                });
            }
        }
    };
}

impl_device!(lua, AirFilter);
impl_device!(lua, AudioSetup);
impl_device!(lua, ContactSensor);
impl_device!(lua, DebugBridge);
impl_device!(lua, HueBridge);
impl_device!(lua, HueGroup);
impl_device!(lua, IkeaOutlet);
impl_device!(lua, KasaOutlet);
impl_device!(lua, LightSensor);
impl_device!(lua, Ntfy);
impl_device!(lua, Presence);
impl_device!(lua, WakeOnLAN);
impl_device!(lua, Washer);

pub fn register_with_lua(lua: &mlua::Lua) -> mlua::Result<()> {
    register_device!(lua, AirFilter);
    register_device!(lua, AudioSetup);
    register_device!(lua, ContactSensor);
    register_device!(lua, DebugBridge);
    register_device!(lua, HueBridge);
    register_device!(lua, HueGroup);
    register_device!(lua, IkeaOutlet);
    register_device!(lua, KasaOutlet);
    register_device!(lua, LightSensor);
    register_device!(lua, Ntfy);
    register_device!(lua, Presence);
    register_device!(lua, WakeOnLAN);
    register_device!(lua, Washer);

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
