use std::collections::HashMap;

use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Attribute, DeriveInput, Token, parenthesized};

enum Attr {
    Trait(TraitAttr),
    AddMethods(AddMethodsAttr),
}

impl Attr {
    fn parse(attr: &Attribute) -> syn::Result<Self> {
        let mut parsed = None;
        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("traits") {
                let input;
                _ = parenthesized!(input in meta.input);
                parsed = Some(Attr::Trait(input.parse()?));
            } else if meta.path.is_ident("add_methods") {
                let value = meta.value()?;
                parsed = Some(Attr::AddMethods(value.parse()?));
            } else {
                return Err(syn::Error::new(meta.path.span(), "Unknown attribute"));
            }

            Ok(())
        })?;

        Ok(parsed.expect("Parsed should be set"))
    }
}

struct TraitAttr {
    traits: Traits,
    aliases: Aliases,
}

impl Parse for TraitAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            traits: input.parse()?,
            aliases: input.parse()?,
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
struct Aliases(Vec<syn::Ident>);

impl Aliases {
    fn has_aliases(&self) -> bool {
        !self.0.is_empty()
    }
}

impl Parse for Aliases {
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
            .map(|aliases| aliases.into_iter().collect())
            .map(Self)
    }
}

#[derive(Clone)]
struct AddMethodsAttr(syn::Path);

impl Parse for AddMethodsAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self(input.parse()?))
    }
}

impl ToTokens for AddMethodsAttr {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self(path) = self;

        tokens.extend(quote! {
            #path
        });
    }
}

struct Implementation {
    name: syn::Ident,
    traits: Traits,
    add_methods: Vec<AddMethodsAttr>,
}

impl quote::ToTokens for Implementation {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        let Self {
            name,
            traits,
            add_methods,
        } = &self;

        let interfaces: String = traits
            .0
            .iter()
            .map(|tr| format!(", Interface{tr}"))
            .collect();

        tokens.extend(quote! {
            impl mlua::UserData for #name {
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

                    ::automation_lib::lua::traits::Device::add_methods(methods);

                    #traits

                    #(
                        #add_methods(methods);
                    )*
                }
            }

            impl ::lua_typed::Typed for #name {
                fn type_name() -> String {
                    stringify!(#name).into()
                }

                fn generate_header() -> std::option::Option<::std::string::String> {
                    let type_name = <Self as ::lua_typed::Typed>::type_name();
                    Some(format!("---@class {type_name}: InterfaceDevice{}\nlocal {type_name}\n", #interfaces))
                }

                fn generate_members() -> Option<String> {
                    let mut output = String::new();

                    let type_name = <Self as ::lua_typed::Typed>::type_name();
                    output += &format!("devices.{type_name} = {{}}\n");
                    let config_name = <<Self as ::automation_lib::device::LuaDeviceCreate>::Config as ::lua_typed::Typed>::type_name();
                    output += &format!("---@param config {config_name}\n");
                    output += &format!("---@return {type_name}\n");
                    output += &format!("function devices.{type_name}.new(config) end\n");


                    Some(output)
                }
            }
        });
    }
}

struct Implementations(Vec<Implementation>);

impl Implementations {
    fn from_attr(attributes: Vec<Attr>, name: syn::Ident) -> Self {
        let mut add_methods = Vec::new();
        let mut all = Traits::default();
        let mut implementations: HashMap<_, Traits> = HashMap::new();
        for attribute in attributes {
            match attribute {
                Attr::Trait(attribute) => {
                    if attribute.aliases.has_aliases() {
                        for alias in &attribute.aliases.0 {
                            implementations
                                .entry(Some(alias.clone()))
                                .or_default()
                                .extend(&attribute.traits);
                        }
                    } else {
                        all.extend(&attribute.traits);
                    }
                }
                Attr::AddMethods(attribute) => add_methods.push(attribute),
            }
        }

        if implementations.is_empty() {
            implementations.entry(None).or_default().extend(&all);
        } else {
            for traits in implementations.values_mut() {
                traits.extend(&all);
            }
        }

        Self(
            implementations
                .into_iter()
                .map(|(alias, traits)| Implementation {
                    name: alias.unwrap_or(name.clone()),
                    traits,
                    add_methods: add_methods.clone(),
                })
                .collect(),
        )
    }
}

pub fn device(input: DeriveInput) -> TokenStream2 {
    let Implementations(imp) = match input
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("device"))
        .map(Attr::parse)
        .try_collect::<Vec<_>>()
    {
        Ok(attr) => Implementations::from_attr(attr, input.ident),
        Err(err) => return err.into_compile_error(),
    };

    quote! {
        #(
            #imp
        )*
    }
}
