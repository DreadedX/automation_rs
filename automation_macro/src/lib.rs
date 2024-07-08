#![feature(let_chains)]
#![feature(iter_intersperse)]
mod lua_device;
mod lua_device_config;

use lua_device::impl_lua_device_macro;
use lua_device_config::impl_lua_device_config_macro;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(LuaDevice, attributes(config))]
pub fn lua_device_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    impl_lua_device_macro(&ast).into()
}

#[proc_macro_derive(LuaDeviceConfig, attributes(device_config))]
pub fn lua_device_config_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    impl_lua_device_config_macro(&ast).into()
}
