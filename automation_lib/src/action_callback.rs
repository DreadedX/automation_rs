use std::marker::PhantomData;

use futures::future::try_join_all;
use mlua::{FromLua, IntoLua, LuaSerdeExt};
use serde::Serialize;

#[derive(Debug, Clone)]
struct Internal {
    callbacks: Vec<mlua::Function>,
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
        let callbacks = match value {
            mlua::Value::Function(f) => vec![f],
            mlua::Value::Table(table) => table
                .pairs::<mlua::Value, mlua::Function>()
                .map(|pair| {
                    let (_, f) = pair?;

                    Ok::<_, mlua::Error>(f)
                })
                .try_collect()?,
            _ => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: value.type_name(),
                    to: "ActionCallback".into(),
                    message: Some("expected function or table of functions".into()),
                });
            }
        };

        Ok(ActionCallback {
            internal: Some(Internal {
                callbacks,
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

        try_join_all(
            internal
                .callbacks
                .iter()
                .map(async |f| f.call_async::<()>((this.clone(), state.clone())).await),
        )
        .await
        .unwrap();
    }

    pub fn is_set(&self) -> bool {
        self.internal.is_some()
    }
}
