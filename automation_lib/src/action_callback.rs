use std::marker::PhantomData;

use mlua::{FromLua, IntoLua, LuaSerdeExt};
use serde::Serialize;

#[derive(Debug, Clone)]
struct Internal {
    value: mlua::Value,
    lua: mlua::Lua,
}

#[derive(Debug, Clone)]
pub struct ActionCallback<T, S> {
    internal: Option<Internal>,
    _this: PhantomData<T>,
    _state: PhantomData<S>,
}

impl<T, S> Default for ActionCallback<T, S> {
    fn default() -> Self {
        Self {
            internal: None,
            _this: PhantomData::<T>,
            _state: PhantomData::<S>,
        }
    }
}

impl<T, S> FromLua for ActionCallback<T, S> {
    fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
        Ok(ActionCallback {
            internal: Some(Internal {
                value,
                lua: lua.clone(),
            }),
            _this: PhantomData::<T>,
            _state: PhantomData::<S>,
        })
    }
}

// TODO: Return proper error here
impl<T, S> ActionCallback<T, S>
where
    T: IntoLua + Sync + Send + Clone + 'static,
    S: Serialize,
{
    pub async fn call(&self, this: &T, state: &S) {
        let Some(internal) = self.internal.as_ref() else {
            return;
        };

        let state = internal.lua.to_value(state).unwrap();

        match &internal.value {
            mlua::Value::Function(f) => f.call_async::<()>((this.clone(), state)).await.unwrap(),
            _ => todo!("Only functions are currently supported"),
        }
    }

    pub fn is_set(&self) -> bool {
        self.internal.is_some()
    }
}
