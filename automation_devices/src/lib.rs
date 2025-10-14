#![feature(iter_intersperse)]
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
            <$device as ::lua_typed::Typed>::type_name,
            ::mlua::Lua::create_proxy::<$device>
        ));

        crate::register_type!($device);
    };
}

pub(crate) use register_device;

type DeviceNameFn = fn() -> String;
type RegisterDeviceFn = fn(lua: &mlua::Lua) -> mlua::Result<mlua::AnyUserData>;

pub struct RegisteredDevice {
    name_fn: DeviceNameFn,
    register_fn: RegisterDeviceFn,
}

impl RegisteredDevice {
    pub const fn new(name_fn: DeviceNameFn, register_fn: RegisterDeviceFn) -> Self {
        Self {
            name_fn,
            register_fn,
        }
    }

    pub fn get_name(&self) -> String {
        (self.name_fn)()
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
        let name = device.get_name();
        debug!(name, "Registering device");
        let proxy = device.register(lua)?;
        devices.set(name, proxy)?;
    }

    Ok(devices)
}

inventory::submit! {Module::new("devices", create_module)}

macro_rules! register_type {
    ($ty:ty) => {
        ::inventory::submit!(crate::RegisteredType(
            <$ty as ::lua_typed::Typed>::generate_full
        ));
    };
}

pub(crate) use register_type;

type RegisterTypeFn = fn() -> Option<String>;

pub struct RegisteredType(RegisterTypeFn);

inventory::collect!(RegisteredType);

pub fn generate_definitions() {
    println!("---@meta\n\nlocal devices\n");
    for ty in inventory::iter::<RegisteredType> {
        let def = ty.0().unwrap();
        println!("{def}");
    }
    println!("return devices")
}
