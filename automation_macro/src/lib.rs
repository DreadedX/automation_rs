#![feature(iter_intersperse)]
mod impl_device;
mod lua_device_config;

use lua_device_config::impl_lua_device_config_macro;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

use crate::impl_device::impl_device_macro;

#[proc_macro_derive(LuaDeviceConfig, attributes(device_config))]
pub fn lua_device_config_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    impl_lua_device_config_macro(&ast).into()
}

#[proc_macro_derive(LuaDevice, attributes(traits))]
pub fn impl_device(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    impl_device_macro(&ast).into()
}

#[proc_macro_derive(LuaSerialize, attributes(traits))]
pub fn lua_serialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    let name = &ast.ident;

    quote! {
        impl ::mlua::IntoLua for #name {
            fn into_lua(self, lua: &::mlua::Lua) -> ::mlua::Result<::mlua::Value> {
                ::mlua::LuaSerdeExt::to_value(lua, &self)
            }
        }
    }
    .into()
}
