use std::marker::PhantomData;

use mlua::{FromLua, IntoLua};

#[derive(Debug, Clone)]
pub struct ActionCallback<T> {
    uuid: uuid::Uuid,
    lua: mlua::Lua,
    phantom: PhantomData<T>,
}

impl<T> FromLua for ActionCallback<T> {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let uuid = uuid::Uuid::new_v4();
        lua.set_named_registry_value(&uuid.to_string(), value)?;

        Ok(ActionCallback {
            uuid,
            lua: lua.clone(),
            phantom: PhantomData::<T>,
        })
    }
}

// TODO: Return proper error here
impl<T> ActionCallback<T>
where
    T: IntoLua + Sync + Send + Clone + Copy + 'static,
{
    pub async fn call(&self, state: T) {
        let uuid = self.uuid;

        let callback: mlua::Value = self.lua.named_registry_value(&uuid.to_string()).unwrap();
        match callback {
            mlua::Value::Function(f) => f.call_async::<()>(state).await.unwrap(),
            _ => todo!("Only functions are currently supported"),
        }
    }
}
