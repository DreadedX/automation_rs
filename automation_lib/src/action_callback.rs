use std::marker::PhantomData;

use mlua::{FromLua, IntoLuaMulti};

#[derive(Debug, Clone)]
struct Internal {
    uuid: uuid::Uuid,
    lua: mlua::Lua,
}

#[derive(Debug, Clone)]
pub struct ActionCallback<T> {
    internal: Option<Internal>,
    phantom: PhantomData<T>,
}

impl<T> Default for ActionCallback<T> {
    fn default() -> Self {
        Self {
            internal: None,
            phantom: PhantomData::<T>,
        }
    }
}

impl<T> FromLua for ActionCallback<T> {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        let uuid = uuid::Uuid::new_v4();
        lua.set_named_registry_value(&uuid.to_string(), value)?;

        Ok(ActionCallback {
            internal: Some(Internal {
                uuid,
                lua: lua.clone(),
            }),
            phantom: PhantomData::<T>,
        })
    }
}

// TODO: Return proper error here
impl<T> ActionCallback<T>
where
    T: IntoLuaMulti + Sync + Send + Clone + 'static,
{
    pub async fn call(&self, state: T) {
        let Some(internal) = self.internal.as_ref() else {
            return;
        };

        let callback: mlua::Value = internal
            .lua
            .named_registry_value(&internal.uuid.to_string())
            .unwrap();
        match callback {
            mlua::Value::Function(f) => f.call_async::<()>(state).await.unwrap(),
            _ => todo!("Only functions are currently supported"),
        }
    }
}
