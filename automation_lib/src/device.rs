use std::fmt::Debug;

use automation_cast::Cast;
use dyn_clone::DynClone;
use google_home::traits::OnOff;
use mlua::ObjectLike;

use crate::event::{OnDarkness, OnMqtt, OnNotification, OnPresence};

// TODO: Make this a proper macro
macro_rules! impl_device {
    ($device:ty) => {
        impl mlua::UserData for $device {
            fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
                methods.add_async_function("new", |_lua, config| async {
                    let device: $device = LuaDeviceCreate::create(config)
                        .await
                        .map_err(mlua::ExternalError::into_lua_err)?;

                    Ok(device)
                });

                methods.add_method("__box", |_lua, this, _: ()| {
                    let b: Box<dyn Device> = Box::new(this.clone());
                    Ok(b)
                });

                methods.add_async_method("get_id", |_lua, this, _: ()| async move { Ok(this.get_id()) });

                if impls::impls!($device: google_home::traits::OnOff) {
                    methods.add_async_method("set_on", |_lua, this, on: bool| async move {
                        (this.deref().cast() as Option<&dyn google_home::traits::OnOff>)
                            .expect("Cast should be valid")
                            .set_on(on)
                            .await
                            .unwrap();

                        Ok(())
                    });

                    methods.add_async_method("is_on", |_lua, this, _: ()| async move {
                        Ok((this.deref().cast() as Option<&dyn google_home::traits::OnOff>)
                            .expect("Cast should be valid")
                            .on()
                            .await
                            .unwrap())
                    });
                }
            }
        }
    };
}
pub(crate) use impl_device;

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
