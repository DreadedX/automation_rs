#![feature(iter_intersperse)]
#![feature(iterator_try_collect)]
mod device;
mod lua_device_config;

use lua_device_config::impl_lua_device_config_macro;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

#[proc_macro_derive(LuaDeviceConfig, attributes(device_config))]
pub fn lua_device_config_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    impl_lua_device_config_macro(&ast).into()
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

/// Derive macro generating an impl for the trait `::mlua::UserData`
///
/// # Device traits
/// The `device(traits)` attribute can be used to tell the macro what traits are implemented so that
/// the appropriate methods can automatically be registered.
/// If the struct does not have any type parameters the syntax is very simple:
/// ```rust
/// #[device(traits(TraitA, TraitB))]
/// ```
///
/// If the type does have type parameters you will have to manually specify all variations that
/// have the trait available:
/// ```rust
/// #[device(traits(TraitA, TraitB for <StateA>, <StateB>))]
/// ```
/// If multiple of these attributes are specified they will all combined appropriately.
///
///
/// ## NOTE
/// If your type _has_ type parameters any instance of the traits attribute that does not specify
/// any type parameters will have the traits applied to _all_ other type parameter variations
/// listed in the other trait attributes. This behavior only applies if there is at least one
/// instance with type parameters specified.
///
/// # Additional methods
/// Additional methods can be added by using the `device(add_methods)` attribute. This attribute
/// takes the path to a function with the following signature that can register the additional methods:
///
/// ```rust
/// # struct D;
/// fn top_secret<M: mlua::UserDataMethods<D>>(methods: &mut M) {}
/// ```
/// It can then be registered with:
/// ```rust
/// #[device(add_methods(top_secret))]
/// ```
#[proc_macro_derive(Device, attributes(device))]
pub fn device(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    device::device(ast).into()
}
