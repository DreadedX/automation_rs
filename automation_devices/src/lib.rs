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
use zigbee::light::{LightBrightness, LightColorTemperature, LightOnOff};
use zigbee::outlet::{OutletOnOff, OutletPower};

pub use self::air_filter::AirFilter;
pub use self::contact_sensor::ContactSensor;
pub use self::hue_bridge::HueBridge;
pub use self::hue_group::HueGroup;
pub use self::hue_switch::HueSwitch;
pub use self::ikea_remote::IkeaRemote;
pub use self::kasa_outlet::KasaOutlet;
pub use self::light_sensor::LightSensor;
pub use self::ntfy::*;
pub use self::presence::Presence;
pub use self::wake_on_lan::WakeOnLAN;
pub use self::washer::Washer;

macro_rules! register_device {
    ($lua:expr, $table:expr, $device:ty) => {
        $table.set(stringify!($device), $lua.create_proxy::<$device>()?)?;
    };
}

pub fn create_module(lua: &mlua::Lua) -> mlua::Result<mlua::Table> {
    let devices = lua.create_table()?;

    register_device!(lua, devices, AirFilter);
    register_device!(lua, devices, ContactSensor);
    register_device!(lua, devices, HueBridge);
    register_device!(lua, devices, HueGroup);
    register_device!(lua, devices, HueSwitch);
    register_device!(lua, devices, IkeaRemote);
    register_device!(lua, devices, KasaOutlet);
    register_device!(lua, devices, LightBrightness);
    register_device!(lua, devices, LightColorTemperature);
    register_device!(lua, devices, LightOnOff);
    register_device!(lua, devices, LightSensor);
    register_device!(lua, devices, Ntfy);
    register_device!(lua, devices, OutletOnOff);
    register_device!(lua, devices, OutletPower);
    register_device!(lua, devices, Presence);
    register_device!(lua, devices, WakeOnLAN);
    register_device!(lua, devices, Washer);

    Ok(devices)
}

inventory::submit! {Module::new("devices", create_module)}
