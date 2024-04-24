use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(LuaDevice, attributes(config))]
pub fn lua_device_derive(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    impl_lua_device_macro(&ast)
}

fn impl_lua_device_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let name_string = name.to_string();
    // TODO: Handle errors properly
    // This includes making sure one, and only one config is specified
    let config = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
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
                lua.globals().set(#name_string, lua.create_proxy::<#name>()?)
            }
        }
        impl mlua::UserData for #name {
            fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
                methods.add_function("new", |lua, config: mlua::Value| {
                    let config: #config = mlua::LuaSerdeExt::from_value(lua, config)?;
                    let config: Box<dyn crate::device_manager::DeviceConfig> = Box::new(config);
                    Ok(config)
                });
            }
        }
    };

    gen.into()
}
