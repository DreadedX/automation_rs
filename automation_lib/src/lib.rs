#![allow(incomplete_features)]
#![feature(iterator_try_collect)]

use tracing::debug;

pub mod action_callback;
pub mod config;
pub mod device;
pub mod device_manager;
pub mod error;
pub mod event;
pub mod helpers;
pub mod lua;
pub mod messages;
pub mod mqtt;
pub mod schedule;

type RegisterFn = fn(lua: &mlua::Lua) -> mlua::Result<mlua::Table>;

pub struct Module {
    name: &'static str,
    register_fn: RegisterFn,
}

impl Module {
    pub const fn new(name: &'static str, register_fn: RegisterFn) -> Self {
        Self { name, register_fn }
    }

    pub const fn get_name(&self) -> &'static str {
        self.name
    }

    pub fn register(&self, lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
        (self.register_fn)(lua)
    }
}

pub fn load_modules(lua: &mlua::Lua) -> mlua::Result<()> {
    debug!("Loading modules...");
    for module in inventory::iter::<Module> {
        debug!(name = module.get_name(), "Registering");
        let table = module.register(lua)?;
        lua.register_module(module.get_name(), table)?;
    }

    Ok(())
}

inventory::collect!(Module);
