use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DataStruct, DeriveInput, Fields, FieldsNamed};

pub fn impl_lua_device_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    // TODO: Handle errors properly
    // This includes making sure one, and only one config is specified
    let config = if let Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { ref named, .. }),
        ..
    }) = ast.data
    {
        named
            .iter()
            .find(|&field| {
                field
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("config"))
            })
            .map(|field| field.ty.clone())
            .unwrap()
    } else {
        unimplemented!()
    };

    let gen = quote! {
        impl #name {
            pub fn register_with_lua(lua: &mlua::Lua) -> mlua::Result<()> {
                lua.globals().set(stringify!(#name), lua.create_proxy::<#name>()?)
            }
        }
        impl mlua::UserData for #name {
            fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
                methods.add_function("new", |lua, config: mlua::Value| {
                    let config: #config = mlua::FromLua::from_lua(config, lua)?;
                    let config: Box<dyn crate::device_manager::DeviceConfig> = Box::new(config);
                    Ok(config)
                });
            }
        }
    };

    gen
}
