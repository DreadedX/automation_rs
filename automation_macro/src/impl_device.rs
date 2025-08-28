use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{Path, Token, parse_macro_input};

struct ImplDevice {
    ty: Path,
    impls: Option<Punctuated<Path, Token![,]>>,
}

impl Parse for ImplDevice {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let ty = input.parse()?;
        let impls = if input.peek(Token![->]) {
            input.parse::<Token![->]>()?;
            Some(input.parse_terminated(Path::parse, Token![,])?)
        } else {
            None
        };

        Ok(ImplDevice { ty, impls })
    }
}

pub fn impl_device_macro(input: proc_macro::TokenStream) -> TokenStream {
    let ImplDevice { ty, impls } = parse_macro_input!(input as ImplDevice);

    let impls: Vec<_> = impls
        .iter()
        .flatten()
        .map(|i| {
            let ident = i
                .segments
                .last()
                .expect("There should be at least one segment")
                .ident
                .clone();

            quote! {
                ::automation_lib::lua::traits::#ident::add_methods(methods);
            }
        })
        .collect();

    quote! {
        impl mlua::UserData for #ty {
            fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
                methods.add_async_function("new", |_lua, config| async {
                    let device: #ty = LuaDeviceCreate::create(config)
                        .await
                        .map_err(mlua::ExternalError::into_lua_err)?;

                    Ok(device)
                });

                methods.add_method("__box", |_lua, this, _: ()| {
                    let b: Box<dyn Device> = Box::new(this.clone());
                    Ok(b)
                });

                methods.add_async_method("get_id", |_lua, this, _: ()| async move { Ok(this.get_id()) });

                #(
                    #impls
                )*
            }
        }
    }.into()
}
