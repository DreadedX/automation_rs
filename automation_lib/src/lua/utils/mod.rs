mod timeout;

use std::time::{SystemTime, UNIX_EPOCH};

pub use timeout::Timeout;

use crate::Module;

fn create_module(lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
    let utils = lua.create_table()?;

    utils.set("Timeout", lua.create_proxy::<Timeout>()?)?;

    let get_hostname = lua.create_function(|_lua, ()| {
        hostname::get()
            .map(|name| name.to_str().unwrap_or("unknown").to_owned())
            .map_err(mlua::ExternalError::into_lua_err)
    })?;
    utils.set("get_hostname", get_hostname)?;
    let get_epoch = lua.create_function(|_lua, ()| {
        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time is after UNIX EPOCH")
            .as_millis())
    })?;
    utils.set("get_epoch", get_epoch)?;

    Ok(utils)
}

inventory::submit! {Module::new("automation:utils", create_module)}
