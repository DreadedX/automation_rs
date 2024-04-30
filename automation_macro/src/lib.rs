mod lua_device;
mod lua_device_config;
mod lua_type_definition;

use lua_device::impl_lua_device_macro;
use lua_device_config::impl_lua_device_config_macro;
use lua_type_definition::impl_lua_type_definition;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(LuaDevice)]
pub fn lua_device_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    impl_lua_device_macro(&ast).into()
}

#[proc_macro_derive(LuaDeviceConfig, attributes(device_config))]
pub fn lua_device_config_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    impl_lua_device_config_macro(&ast).into()
}

#[proc_macro_derive(LuaTypeDefinition, attributes(device_config))]
pub fn lua_type_definition_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    impl_lua_type_definition(&ast).into()
}
