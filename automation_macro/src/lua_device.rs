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
                methods.add_async_function("new", |lua, config: mlua::Value| async {
                    let config: #config = mlua::FromLua::from_lua(config, lua)?;
                    let device = #name::create(config).await.map_err(mlua::ExternalError::into_lua_err)?;

                    Ok(crate::device_manager::WrappedDevice::new(Box::new(device)))
                });
            }
        }
    };

    gen
}
