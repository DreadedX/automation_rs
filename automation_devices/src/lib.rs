mod air_filter;
mod contact_sensor;
mod debug_bridge;
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

use automation_lib::device::{Device, LuaDeviceCreate};
use zigbee::light::{LightBrightness, LightColorTemperature, LightOnOff};
use zigbee::outlet::{OutletOnOff, OutletPower};

pub use self::air_filter::AirFilter;
pub use self::contact_sensor::ContactSensor;
pub use self::debug_bridge::DebugBridge;
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
    ($lua:expr, $device:ty) => {
        $lua.globals()
            .set(stringify!($device), $lua.create_proxy::<$device>()?)?;
    };
}

pub fn register_with_lua(lua: &mlua::Lua) -> mlua::Result<()> {
    register_device!(lua, AirFilter);
    register_device!(lua, ContactSensor);
    register_device!(lua, DebugBridge);
    register_device!(lua, HueBridge);
    register_device!(lua, HueGroup);
    register_device!(lua, HueSwitch);
    register_device!(lua, IkeaRemote);
    register_device!(lua, KasaOutlet);
    register_device!(lua, LightBrightness);
    register_device!(lua, LightColorTemperature);
    register_device!(lua, LightOnOff);
    register_device!(lua, LightSensor);
    register_device!(lua, Ntfy);
    register_device!(lua, OutletOnOff);
    register_device!(lua, OutletPower);
    register_device!(lua, Presence);
    register_device!(lua, WakeOnLAN);
    register_device!(lua, Washer);

    Ok(())
}
