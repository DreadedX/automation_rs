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
use dyn_clone::DynClone;
use google_home::traits::OnOff;
use mlua::AnyUserDataExt;

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
    ($device:ty) => {
        impl mlua::UserData for $device {
            fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
                methods.add_async_function("new", |_lua, config| async {
                    let device: $device = crate::devices::LuaDeviceCreate::create(config)
                        .await
                        .map_err(mlua::ExternalError::into_lua_err)?;

                    Ok(device)
                });

                methods.add_method("__box", |_lua, this, _: ()| {
                    let b: Box<dyn Device> = Box::new(this.clone());
                    Ok(b)
                });

                methods.add_async_method("get_id", |_lua, this, _: ()| async { Ok(this.get_id()) });

                if impls::impls!($device: OnOff) {
                    methods.add_async_method("set_on", |_lua, this, on: bool| async move {
                        (this.cast() as Option<&dyn OnOff>)
                            .unwrap()
                            .set_on(on)
                            .await
                            .unwrap();

                        Ok(())
                    });

                    methods.add_async_method("is_on", |_lua, this, _: ()| async move {
                        Ok((this.cast() as Option<&dyn OnOff>)
                            .unwrap()
                            .on()
                            .await
                            .unwrap())
                    });
                }
            }
        }
    };
}

impl_device!(AirFilter);
impl_device!(AudioSetup);
impl_device!(ContactSensor);
impl_device!(DebugBridge);
impl_device!(HueBridge);
impl_device!(HueGroup);
impl_device!(IkeaOutlet);
impl_device!(KasaOutlet);
impl_device!(LightSensor);
impl_device!(Ntfy);
impl_device!(Presence);
impl_device!(WakeOnLAN);
impl_device!(Washer);

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
    + DynClone
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

impl<'lua> mlua::FromLua<'lua> for Box<dyn Device> {
    fn from_lua(value: mlua::Value<'lua>, _lua: &'lua mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => {
                let ud = if ud.is::<Box<dyn Device>>() {
                    ud
                } else {
                    ud.call_method::<_, mlua::AnyUserData>("__box", ())?
                };

                let b = ud.borrow::<Self>()?.clone();
                Ok(b)
            }
            _ => Err(mlua::Error::RuntimeError("Expected user data".into())),
        }
    }
}
impl mlua::UserData for Box<dyn Device> {}

dyn_clone::clone_trait_object!(Device);
