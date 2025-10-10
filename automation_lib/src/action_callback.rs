use std::marker::PhantomData;

use futures::future::try_join_all;
use lua_typed::Typed;
use mlua::{FromLua, IntoLuaMulti};

#[derive(Debug, Clone)]
pub struct ActionCallback<P> {
    callbacks: Vec<mlua::Function>,
    _parameters: PhantomData<P>,
}

impl<A: Typed> Typed for ActionCallback<A> {
    fn type_name() -> String {
        let type_name = A::type_name();
        format!("fun(_: {type_name}) | fun(_: {type_name})[]")
    }
}

impl<A: Typed, B: Typed> Typed for ActionCallback<(A, B)> {
    fn type_name() -> String {
        let type_name_a = A::type_name();
        let type_name_b = B::type_name();
        format!(
            "fun(_: {type_name_a}, _: {type_name_b}) | fun(_: {type_name_a}, _: {type_name_b})[]"
        )
    }
}

// NOTE: For some reason the derive macro combined with PhantomData leads to issues where it
// requires all types part of P to implement default, even if they never actually get constructed.
// By manually implemented Default it works fine.
impl<P> Default for ActionCallback<P> {
    fn default() -> Self {
        Self {
            callbacks: Default::default(),
            _parameters: Default::default(),
        }
    }
}

impl<P> FromLua for ActionCallback<P> {
    fn from_lua(value: mlua::Value, _lua: &mlua::Lua) -> mlua::Result<Self> {
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
            callbacks,
            _parameters: PhantomData::<P>,
        })
    }
}

// TODO: Return proper error here
impl<P> ActionCallback<P>
where
    P: IntoLuaMulti + Sync + Clone,
{
    pub async fn call(&self, parameters: P) {
        try_join_all(
            self.callbacks
                .iter()
                .map(async |f| f.call_async::<()>(parameters.clone()).await),
        )
        .await
        .unwrap();
    }

    pub fn is_empty(&self) -> bool {
        self.callbacks.is_empty()
    }
}
