use std::fmt::Debug;

use automation_cast::Cast;
use dyn_clone::DynClone;
use google_home::traits::OnOff;
use mlua::ObjectLike;

use crate::event::{OnDarkness, OnMqtt, OnNotification, OnPresence};

#[async_trait::async_trait]
pub trait LuaDeviceCreate {
    type Config;
    type Error;

    async fn create(config: Self::Config) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

pub trait Device:
    Debug
    + DynClone
    + Sync
    + Send
    + Cast<dyn google_home::Device>
    + Cast<dyn OnMqtt>
    + Cast<dyn OnPresence>
    + Cast<dyn OnDarkness>
    + Cast<dyn OnNotification>
    + Cast<dyn OnOff>
{
    fn get_id(&self) -> String;
}

impl mlua::FromLua for Box<dyn Device> {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => {
                let ud = if ud.is::<Box<dyn Device>>() {
                    ud
                } else {
                    ud.call_method::<_>("__box", ())?
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
