mod timeout;

pub use timeout::Timeout;

pub fn register_with_lua(lua: &mlua::Lua) -> mlua::Result<()> {
    lua.globals()
        .set("Timeout", lua.create_proxy::<Timeout>()?)?;

    Ok(())
}
