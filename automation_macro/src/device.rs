use std::collections::HashMap;

use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, DeriveInput, Token, parenthesized};

enum Attr {
    Trait(TraitAttr),
}

impl Parse for Attr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let ident: syn::Ident = input.parse()?;

        let attr;
        _ = parenthesized!(attr in input);

        let attr = match ident.to_string().as_str() {
            "traits" => Attr::Trait(attr.parse()?),
            _ => return Err(syn::Error::new(ident.span(), "Expected 'traits'")),
        };

        Ok(attr)
    }
}

struct TraitAttr {
    traits: Traits,
    generics: Generics,
}

impl Parse for TraitAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            traits: input.parse()?,
            generics: input.parse()?,
        })
    }
}

#[derive(Default)]
struct Traits(Vec<syn::Ident>);

impl Traits {
    fn extend(&mut self, other: &Traits) {
        self.0.extend_from_slice(&other.0);
    }
}

impl Parse for Traits {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        input
            .call(Punctuated::<_, Token![,]>::parse_separated_nonempty)
            .map(|traits| traits.into_iter().collect())
            .map(Self)
    }
}

impl ToTokens for Traits {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self(traits) = &self;

        tokens.extend(quote! {
            #(
                ::automation_lib::lua::traits::#traits::add_methods(methods);
            )*
        });
    }
}

#[derive(Default)]
struct Generics(Vec<syn::AngleBracketedGenericArguments>);

impl Generics {
    fn has_generics(&self) -> bool {
        !self.0.is_empty()
    }
}

impl Parse for Generics {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        if !input.peek(Token![for]) {
            if input.is_empty() {
                return Ok(Default::default());
            } else {
                return Err(input.error("Expected ')' or 'for'"));
            }
        }

        _ = input.parse::<syn::Token![for]>()?;

        input
            .call(Punctuated::<_, Token![,]>::parse_separated_nonempty)
            .map(|generics| generics.into_iter().collect())
            .map(Self)
    }
}

struct Implementation {
    generics: Option<syn::AngleBracketedGenericArguments>,
    traits: Traits,
}

impl From<(Option<syn::AngleBracketedGenericArguments>, Traits)> for Implementation {
    fn from(value: (Option<syn::AngleBracketedGenericArguments>, Traits)) -> Self {
        Self {
            generics: value.0,
            traits: value.1,
        }
    }
}

impl quote::ToTokens for Implementation {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self { generics, traits } = &self;

        tokens.extend(quote! {
            #generics {
                fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
                    methods.add_async_function("new", async |_lua, config| {
                        let device: Self = LuaDeviceCreate::create(config)
                            .await
                            .map_err(mlua::ExternalError::into_lua_err)?;

                        Ok(device)
                    });

                    methods.add_method("__box", |_lua, this, _: ()| {
                        let b: Box<dyn Device> = Box::new(this.clone());
                        Ok(b)
                    });

                    methods.add_async_method("get_id", async |_lua, this, _: ()| { Ok(this.get_id()) });

					#traits
                }
            }
        });
    }
}

struct Implementations(Vec<Implementation>);

impl From<Vec<Attr>> for Implementations {
    fn from(attributes: Vec<Attr>) -> Self {
        let mut all = Traits::default();
        let mut implementations: HashMap<_, Traits> = HashMap::new();
        for attribute in attributes {
            match attribute {
                Attr::Trait(attribute) => {
                    if attribute.generics.has_generics() {
                        for generic in &attribute.generics.0 {
                            implementations
                                .entry(Some(generic.clone()))
                                .or_default()
                                .extend(&attribute.traits);
                        }
                    } else {
                        all.extend(&attribute.traits);
                    }
                }
            }
        }

        if implementations.is_empty() {
            implementations.entry(None).or_default().extend(&all);
        } else {
            for traits in implementations.values_mut() {
                traits.extend(&all);
            }
        }

        Self(implementations.into_iter().map(Into::into).collect())
    }
}

pub fn device(input: &DeriveInput) -> TokenStream2 {
    let name = &input.ident;

    let Implementations(imp) = match input
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("device"))
        .map(Attribute::parse_args)
        .try_collect::<Vec<_>>()
    {
        Ok(result) => result.into(),
        Err(err) => return err.into_compile_error(),
    };

    quote! {
        #(
            impl mlua::UserData for #name #imp
        )*
    }
}
