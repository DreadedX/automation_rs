mod air_filter;
mod contact_sensor;
mod debug_bridge;
mod hue_bridge;
mod hue_group;
mod hue_switch;
mod ikea_remote;
mod kasa_outlet;
mod light_sensor;
mod wake_on_lan;
mod washer;
mod zigbee;

use std::ops::Deref;

use automation_cast::Cast;
use automation_lib::device::{Device, LuaDeviceCreate};
use zigbee::light::{LightBrightness, LightOnOff};
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
pub use self::wake_on_lan::WakeOnLAN;
pub use self::washer::Washer;

macro_rules! register_device {
    ($lua:expr, $device:ty) => {
        $lua.globals()
            .set(stringify!($device), $lua.create_proxy::<$device>()?)?;
    };
}

macro_rules! impl_device {
    ($device:ty) => {
        impl mlua::UserData for $device {
            fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
                methods.add_async_function("new", |_lua, config| async {
                    let device: $device = LuaDeviceCreate::create(config)
                        .await
                        .map_err(mlua::ExternalError::into_lua_err)?;

                    Ok(device)
                });

                methods.add_method("__box", |_lua, this, _: ()| {
                    let b: Box<dyn Device> = Box::new(this.clone());
                    Ok(b)
                });

                methods.add_async_method("get_id", |_lua, this, _: ()| async move { Ok(this.get_id()) });

                if impls::impls!($device: google_home::traits::OnOff) {
                    methods.add_async_method("set_on", |_lua, this, on: bool| async move {
                        (this.deref().cast() as Option<&dyn google_home::traits::OnOff>)
                            .expect("Cast should be valid")
                            .set_on(on)
                            .await
                            .unwrap();

                        Ok(())
                    });

                    methods.add_async_method("on", |_lua, this, _: ()| async move {
                        Ok((this.deref().cast() as Option<&dyn google_home::traits::OnOff>)
                            .expect("Cast should be valid")
                            .on()
                            .await
                            .unwrap())
                    });
                }

                if impls::impls!($device: google_home::traits::Brightness) {
                    methods.add_async_method("set_brightness", |_lua, this, brightness: u8| async move {
                        (this.deref().cast() as Option<&dyn google_home::traits::Brightness>)
                            .expect("Cast should be valid")
                            .set_brightness(brightness)
                            .await
                            .unwrap();

                        Ok(())
                    });

                    methods.add_async_method("brightness", |_lua, this, _: ()| async move {
                        Ok((this.deref().cast() as Option<&dyn google_home::traits::Brightness>)
                            .expect("Cast should be valid")
                            .brightness()
                            .await
                            .unwrap())
                    });
                }

                if impls::impls!($device: google_home::traits::OpenClose) {
					// TODO: Make discrete_only_open_close and query_only_open_close static, that way we can
					// add only the supported functions and drop _percet if discrete is true
					methods.add_async_method("set_open_percent", |_lua, this, open_percent: u8| async move {
						(this.deref().cast() as Option<&dyn google_home::traits::OpenClose>)
							.expect("Cast should be valid")
							.set_open_percent(open_percent)
							.await
							.unwrap();

						Ok(())
					});

                    methods.add_async_method("open_percent", |_lua, this, _: ()| async move {
                        Ok((this.deref().cast() as Option<&dyn google_home::traits::OpenClose>)
                            .expect("Cast should be valid")
                            .open_percent()
                            .await
                            .unwrap())
                    });
                }
            }
        }
    };
}

impl_device!(LightOnOff);
impl_device!(LightBrightness);
impl_device!(OutletOnOff);
impl_device!(OutletPower);
impl_device!(AirFilter);
impl_device!(ContactSensor);
impl_device!(DebugBridge);
impl_device!(HueBridge);
impl_device!(HueGroup);
impl_device!(HueSwitch);
impl_device!(IkeaRemote);
impl_device!(KasaOutlet);
impl_device!(LightSensor);
impl_device!(WakeOnLAN);
impl_device!(Washer);

pub fn register_with_lua(lua: &mlua::Lua) -> mlua::Result<()> {
    register_device!(lua, LightOnOff);
    register_device!(lua, LightBrightness);
    register_device!(lua, OutletOnOff);
    register_device!(lua, OutletPower);
    register_device!(lua, AirFilter);
    register_device!(lua, ContactSensor);
    register_device!(lua, DebugBridge);
    register_device!(lua, HueBridge);
    register_device!(lua, HueGroup);
    register_device!(lua, HueSwitch);
    register_device!(lua, IkeaRemote);
    register_device!(lua, KasaOutlet);
    register_device!(lua, LightSensor);
    register_device!(lua, WakeOnLAN);
    register_device!(lua, Washer);

    Ok(())
}
