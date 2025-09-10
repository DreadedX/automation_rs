mod air_filter;
mod contact_sensor;
mod hue_bridge;
mod hue_group;
mod hue_switch;
mod ikea_remote;
mod kasa_outlet;
mod light_sensor;
mod ntfy;
mod presence;
mod wake_on_lan;
mod washer;
mod zigbee;

use automation_lib::Module;
use automation_lib::device::{Device, LuaDeviceCreate};
use tracing::debug;

macro_rules! register_device {
    ($device:ty) => {
        ::inventory::submit!(crate::RegisteredDevice::new(
            stringify!($device),
            ::mlua::Lua::create_proxy::<$device>
        ));
    };
}

pub(crate) use register_device;

type RegisterFn = fn(lua: &mlua::Lua) -> mlua::Result<mlua::AnyUserData>;

pub struct RegisteredDevice {
    name: &'static str,
    register_fn: RegisterFn,
}

impl RegisteredDevice {
    pub const fn new(name: &'static str, register_fn: RegisterFn) -> Self {
        Self { name, register_fn }
    }

    pub const fn get_name(&self) -> &'static str {
        self.name
    }

    pub fn register(&self, lua: &mlua::Lua) -> mlua::Result<mlua::AnyUserData> {
        (self.register_fn)(lua)
    }
}

inventory::collect!(RegisteredDevice);

pub fn create_module(lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
    let devices = lua.create_table()?;

    debug!("Loading devices...");
    for device in inventory::iter::<RegisteredDevice> {
        debug!(name = device.get_name(), "Registering device");
        let proxy = device.register(lua)?;
        devices.set(device.get_name(), proxy)?;
    }

    Ok(devices)
}

inventory::submit! {Module::new("devices", create_module)}
