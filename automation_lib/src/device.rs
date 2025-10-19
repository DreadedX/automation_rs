use std::fmt::Debug;

use automation_cast::Cast;
use dyn_clone::DynClone;
use lua_typed::Typed;
use mlua::ObjectLike;

use crate::event::OnMqtt;

#[async_trait::async_trait]
pub trait LuaDeviceCreate {
    type Config;
    type Error;

    async fn create(config: Self::Config) -> Result<Self, Self::Error>
    where
        Self: Sized;
}

pub trait Device:
    Debug + DynClone + Sync + Send + Cast<dyn google_home::Device> + Cast<dyn OnMqtt>
{
    fn get_id(&self) -> String;
}

impl mlua::FromLua for Box<dyn Device> {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => {
                let ud = if ud.is::<Self>() {
                    ud
                } else {
                    ud.call_method::<_>("__box", ())?
                };

                let b = ud.borrow::<Self>()?.clone();
                Ok(b)
            }
            _ => Err(mlua::Error::runtime(format!(
                "Expected user data, instead found: {}",
                value.type_name()
            ))),
        }
    }
}
impl mlua::UserData for Box<dyn Device> {}

impl Typed for Box<dyn Device> {
    fn type_name() -> String {
        "DeviceInterface".into()
    }
}

dyn_clone::clone_trait_object!(Device);
