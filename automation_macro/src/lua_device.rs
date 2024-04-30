use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

pub fn impl_lua_device_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let gen = quote! {
        impl #name {
            pub fn register_with_lua(lua: &mlua::Lua) -> mlua::Result<()> {
                lua.globals().set(stringify!(#name), lua.create_proxy::<#name>()?)
            }

            pub fn generate_lua_definition() -> String {
                // TODO: Do not hardcode the name of the config type
                let def = format!(
                    r#"--- @class {0}
{0} = {{}}
--- @param config {0}Config
--- @return WrappedDevice
function {0}.new(config) end
"#, stringify!(#name)
                );

                def
            }
        }
        impl mlua::UserData for #name {
            fn add_methods<'lua, M: mlua::UserDataMethods<'lua, Self>>(methods: &mut M) {
                methods.add_async_function("new", |lua, config: mlua::Value| async {
                    let config = mlua::FromLua::from_lua(config, lua)?;

                    // TODO: Using crate:: could cause issues
                    let device: #name = crate::devices::LuaDeviceCreate::create(config).await.map_err(mlua::ExternalError::into_lua_err)?;

                    Ok(crate::device_manager::WrappedDevice::new(Box::new(device)))
                });
            }
        }
    };

    gen
}
