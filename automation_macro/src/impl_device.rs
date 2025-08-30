use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{AngleBracketedGenericArguments, Attribute, DeriveInput, Ident, Path, Token};

#[derive(Debug, Default)]
struct Impl {
    generics: Option<AngleBracketedGenericArguments>,
    traits: Vec<Path>,
}

impl Parse for Impl {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let generics = if input.peek(Token![<]) {
            let generics = input.parse()?;
            input.parse::<Token![:]>()?;

            Some(generics)
        } else {
            None
        };

        let traits: Punctuated<_, _> = input.parse_terminated(Path::parse, Token![,])?;
        let traits = traits.into_iter().collect();

        Ok(Impl { generics, traits })
    }
}

impl Impl {
    fn generate(&self, name: &Ident) -> TokenStream {
        let generics = &self.generics;

        // If an identifier is specified, assume it is placed in ::automation_lib::lua::traits,
        // otherwise use the provided path
        let traits = self.traits.iter().map(|t| {
            if let Some(ident) = t.get_ident() {
                quote! {::automation_lib::lua::traits::#ident }
            } else {
                t.to_token_stream()
            }
        });

        quote! {
            impl mlua::UserData for #name #generics {
                fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
                    methods.add_async_function("new", |_lua, config| async {
                        let device: Self = LuaDeviceCreate::create(config)
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
                        #traits::add_methods(methods);
                    )*
                }
            }
        }
    }
}

pub fn impl_device_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;

    let impls: TokenStream = ast
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("traits"))
        .flat_map(Attribute::parse_args::<Impl>)
        .map(|im| im.generate(name))
        .collect();

    if impls.is_empty() {
        Impl::default().generate(name)
    } else {
        impls
    }
}
