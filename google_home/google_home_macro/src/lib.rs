#![feature(let_chains)]
#![feature(iter_intersperse)]
use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::token::Brace;
use syn::{
    braced, parse_macro_input, GenericArgument, Ident, LitStr, Path, PathArguments, PathSegment,
    ReturnType, Signature, Token, Type, TypePath,
};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(required);
}

#[derive(Debug)]
struct FieldAttribute {
    ident: Ident,
    _colon_token: Token![:],
    ty: Type,
}

impl Parse for FieldAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            ident: input.parse()?,
            _colon_token: input.parse()?,
            ty: input.parse()?,
        })
    }
}

#[derive(Debug)]
struct FieldState {
    sign: Signature,
}

impl Parse for FieldState {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            sign: input.parse()?,
        })
    }
}

#[derive(Debug)]
struct FieldExecute {
    name: LitStr,
    _fat_arrow_token: Token![=>],
    sign: Signature,
}

impl Parse for FieldExecute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            name: input.parse()?,
            _fat_arrow_token: input.parse()?,
            sign: input.parse()?,
        })
    }
}

#[derive(Debug)]
enum Field {
    Attribute(FieldAttribute),
    State(FieldState),
    Execute(FieldExecute),
}

impl Parse for Field {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.peek(Ident) {
            Ok(Field::Attribute(input.parse()?))
        } else if input.peek(LitStr) {
            Ok(Field::Execute(input.parse()?))
        } else {
            Ok(Field::State(input.parse()?))
        }
    }
}

#[derive(Debug)]
struct Trait {
    name: LitStr,
    _fat_arrow_token: Token![=>],
    _trait_token: Token![trait],
    ident: Ident,
    _brace_token: Brace,
    fields: Punctuated<Field, Token![,]>,
}

impl Parse for Trait {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(Self {
            name: input.parse()?,
            _fat_arrow_token: input.parse()?,
            _trait_token: input.parse()?,
            ident: input.parse()?,
            _brace_token: braced!(content in input),
            fields: content.parse_terminated(Field::parse, Token![,])?,
        })
    }
}

#[derive(Debug)]
struct Input {
    ty: TypePath,
    _comma: Token![,],
    traits: Punctuated<Trait, Token![,]>,
}

// TODO: Error on duplicate name?
impl Parse for Input {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(Self {
            ty: input.parse()?,
            _comma: input.parse()?,
            traits: input.parse_terminated(Trait::parse, Token![,])?,
        })
    }
}

fn extract_type_path(ty: &syn::Type) -> Option<&Path> {
    match *ty {
        Type::Path(ref typepath) if typepath.qself.is_none() => Some(&typepath.path),
        _ => None,
    }
}

fn extract_segment<'a>(path: &'a Path, options: &[&str]) -> Option<&'a PathSegment> {
    let idents_of_path = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .intersperse('|'.into())
        .collect::<String>();

    options
        .iter()
        .find(|s| &idents_of_path == *s)
        .and_then(|_| path.segments.last())
}

// Based on: https://stackoverflow.com/a/56264023
fn extract_type_from_option(ty: &syn::Type) -> Option<&syn::Type> {
    extract_type_path(ty)
        .and_then(|path| {
            extract_segment(path, &["Option", "std|option|Option", "core|option|Option"])
        })
        .and_then(|path_seg| {
            let type_params = &path_seg.arguments;
            // It should have only on angle-bracketed param ("<String>"):
            match *type_params {
                PathArguments::AngleBracketed(ref params) => params.args.first(),
                _ => None,
            }
        })
        .and_then(|generic_arg| match *generic_arg {
            GenericArgument::Type(ref ty) => Some(ty),
            _ => None,
        })
}

fn extract_type_from_result(ty: &syn::Type) -> Option<&syn::Type> {
    extract_type_path(ty)
        .and_then(|path| {
            extract_segment(path, &["Result", "std|result|Result", "core|result|Result"])
        })
        .and_then(|path_seg| {
            let type_params = &path_seg.arguments;
            // It should have only on angle-bracketed param ("<String>"):
            match *type_params {
                PathArguments::AngleBracketed(ref params) => params.args.first(),
                _ => None,
            }
        })
        .and_then(|generic_arg| match *generic_arg {
            GenericArgument::Type(ref ty) => Some(ty),
            _ => None,
        })
}

fn get_attributes_struct_ident(t: &Trait) -> Ident {
    syn::Ident::new(&format!("{}Attributes", t.ident), t.ident.span())
}

fn get_attributes_struct(t: &Trait) -> proc_macro2::TokenStream {
    let fields = t.fields.iter().filter_map(|f| match f {
        Field::Attribute(attr) => {
            let ident = &attr.ident;
            let ty = &attr.ty;

            // TODO: Extract into function
            if let Some(ty) = extract_type_from_option(ty) {
                Some(quote! {
                    #[serde(skip_serializing_if = "core::option::Option::is_none")]
                    #ident: ::core::option::Option<#ty>
                })
            } else {
                Some(quote! {
                    #ident: #ty
                })
            }
        }
        _ => None,
    });

    let name = get_attributes_struct_ident(t);
    quote! {
        #[derive(Debug, serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct #name {
            #(#fields,)*
        }
    }
}

fn get_state_struct_ident(t: &Trait) -> Ident {
    syn::Ident::new(&format!("{}State", t.ident), t.ident.span())
}

fn get_state_struct(t: &Trait) -> proc_macro2::TokenStream {
    let fields = t.fields.iter().filter_map(|f| match f {
        Field::State(state) => {
            let ident = &state.sign.ident;

            let ReturnType::Type(_, ty) = &state.sign.output else {
                return None;
            };

            let ty = extract_type_from_result(ty).unwrap_or(ty);

            if let Some(ty) = extract_type_from_option(ty) {
                Some(quote! {
                    #[serde(skip_serializing_if = "core::option::Option::is_none")]
                    #ident: ::core::option::Option<#ty>
                })
            } else {
                Some(quote! {#ident: #ty})
            }
        }
        _ => None,
    });

    let name = get_state_struct_ident(t);
    quote! {
        #[derive(Debug, serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct #name {
            #(#fields,)*
        }
    }
}

fn get_command_enum(traits: &Punctuated<Trait, Token![,]>) -> proc_macro2::TokenStream {
    let items = traits.iter().flat_map(|t| {
        t.fields.iter().filter_map(|f| match f {
            Field::Execute(execute) => {
                let name = execute.name.value();
                let ident = Ident::new(
                    name.split_at(name.rfind('.').map(|v| v + 1).unwrap_or(0)).1,
                    execute.name.span(),
                );

                let parameters = execute.sign.inputs.iter().skip(1);

                Some(quote! {
                    #[serde(rename = #name, rename_all = "camelCase")]
                    #ident {
                        #(#parameters,)*
                    }
                })
            }
            _ => None,
        })
    });

    quote! {
        #[derive(Debug, Clone, serde::Deserialize)]
        #[serde(tag = "command", content = "params", rename_all = "camelCase")]
        pub enum Command {
            #(#items,)*
        }
    }
}

fn get_trait_enum(traits: &Punctuated<Trait, Token![,]>) -> proc_macro2::TokenStream {
    let items = traits.iter().map(|t| {
        let name = &t.name;
        let ident = &t.ident;
        quote! {
            #[serde(rename = #name)]
            #ident
        }
    });

    quote! {
        #[derive(Debug, serde::Serialize)]
        pub enum Trait {
            #(#items,)*
        }
    }
}

fn get_trait(t: &Trait) -> proc_macro2::TokenStream {
    let fields = t.fields.iter().map(|f| match f {
        Field::Attribute(attr) => {
            let name = &attr.ident;
            let ty = &attr.ty;

            // If the default type is marked as optional, respond None by default
            if let Some(ty) = extract_type_from_option(ty) {
                quote! {
                    fn #name(&self) -> Option<#ty> {
                        None
                    }
                }
            } else {
                quote! {
                    fn #name(&self) -> #ty;
                }
            }
        }
        Field::State(state) => {
            let sign = &state.sign;

            let ReturnType::Type(_, ty) = &state.sign.output else {
                todo!("Handle weird function return types");
            };

            let inner = extract_type_from_result(ty);
            // If the default type is marked as optional, respond None by default
            if extract_type_from_option(inner.unwrap_or(ty)).is_some() {
                if inner.is_some() {
                    quote! {
                        #sign {
                            Ok(None)
                        }
                    }
                } else {
                    quote! {
                        #sign {
                            None
                        }
                    }
                }
            } else {
                quote! {
                    #sign;
                }
            }
        }
        Field::Execute(execute) => {
            let sign = &execute.sign;
            quote! {
                #sign;
            }
        }
    });

    let ident = &t.ident;

    let attr_ident = get_attributes_struct_ident(t);
    let attr = t.fields.iter().filter_map(|f| match f {
        Field::Attribute(attr) => {
            let name = &attr.ident;

            Some(quote! {
                #name: self.#name()
            })
        }
        _ => None,
    });

    let state_ident = get_state_struct_ident(t);
    let state = t.fields.iter().filter_map(|f| match f {
        Field::State(state) => {
            let ident = &state.sign.ident;
            let f_ident = &state.sign.ident;

            let asyncness = if state.sign.asyncness.is_some() {
                quote! {.await}
            } else {
                quote! {}
            };

            let errors = if let ReturnType::Type(_, ty) = &state.sign.output
                && extract_type_from_result(ty).is_some()
            {
                quote! {?}
            } else {
                quote! {}
            };

            Some(quote! {
                #ident: self.#f_ident() #asyncness #errors,
            })
        }
        _ => None,
    });

    quote! {
        #[async_trait::async_trait]
        pub trait #ident: Sync + Send {
            #(#fields)*

            fn get_attributes(&self) -> #attr_ident {
                #attr_ident { #(#attr,)* }
            }

            async fn get_state(&self) -> Result<#state_ident, Box<dyn ::std::error::Error>> {
                Ok(#state_ident { #(#state)* })
            }
        }
    }
}

#[proc_macro]
pub fn traits(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as Input);
    let traits = input.traits;

    let structs = traits.iter().map(|t| {
        let attr = get_attributes_struct(t);
        let state = get_state_struct(t);
        let tra = get_trait(t);

        quote! {
            #attr
            #state
            #tra
        }
    });

    let command_enum = get_command_enum(&traits);
    let trait_enum = get_trait_enum(&traits);

    let sync = traits.iter().map(|t| {
        let ident = &t.ident;

        quote! {
            if let Some(t) = self.cast() as Option<&dyn #ident> {
                traits.push(Trait::#ident);
                let value = serde_json::to_value(t.get_attributes())?;
                json_value_merge::Merge::merge(&mut attrs, &value);
            }
        }
    });

    let query = traits.iter().map(|t| {
        let ident = &t.ident;

        quote! {
            if let Some(t) = self.cast() as Option<&dyn #ident> {
                let value = serde_json::to_value(t.get_state().await?)?;
                json_value_merge::Merge::merge(&mut state, &value);
            }
        }
    });

    let execute = traits.iter().flat_map(|t| {
        t.fields.iter().filter_map(|f| match f {
            Field::Execute(execute) => {
                let ident = &t.ident;
                let name = execute.name.value();
                let command_name = Ident::new(
                    name.split_at(name.rfind('.').map(|v| v + 1).unwrap_or(0)).1,
                    execute.name.span(),
                );
                let f_name = &&execute.sign.ident;
                let parameters = execute
                    .sign
                    .inputs
                    .iter()
                    .filter_map(|p| {
                        if let syn::FnArg::Typed(p) = p {
                            Some(&p.pat)
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();

                let asyncness = if execute.sign.asyncness.is_some() {
                    quote! {.await}
                } else {
                    quote! {}
                };

                let errors = if let ReturnType::Type(_, ty) = &execute.sign.output
                    && extract_type_from_result(ty).is_some()
                {
                    quote! {?}
                } else {
                    quote! {}
                };

                Some(quote! {
                    Command::#command_name {#(#parameters,)*} => {
                        if let Some(t) = self.cast() as Option<&dyn #ident> {
                            t.#f_name(#(#parameters,)*) #asyncness #errors;
                            serde_json::to_value(t.get_state().await?)?
                        } else {
                            todo!("Device does not support action, return proper error");
                        }
                    }
                })
            }
            _ => None,
        })
    });

    let ty = input.ty;

    let fulfillment = Ident::new(
        &format!("{}Fulfillment", ty.path.segments.last().unwrap().ident),
        ty.path.segments.last().unwrap().ident.span(),
    );

    quote! {
		// TODO: This is always the same, so should not be part of the macro, but instead something
		// else
        #[async_trait::async_trait]
		pub trait #fulfillment: Sync + Send {
			async fn sync(&self) -> Result<(Vec<Trait>, serde_json::Value), Box<dyn ::std::error::Error>>;
			async fn query(&self) -> Result<serde_json::Value, Box<dyn ::std::error::Error>>;
            async fn execute(&self, command: Command) -> Result<serde_json::Value, Box<dyn std::error::Error>>;
		}

		#(#structs)*

		#command_enum
		#trait_enum

        #[async_trait::async_trait]
		impl<D> #fulfillment for D where D: #ty
		{
			async fn sync(&self) -> Result<(Vec<Trait>, serde_json::Value), Box<dyn ::std::error::Error>> {
				let mut traits = Vec::new();
				let mut attrs = serde_json::Value::Null;

				#(#sync)*

				Ok((traits, attrs))
			  }

			async fn query(&self) -> Result<serde_json::Value, Box<dyn ::std::error::Error>> {
				let mut state = serde_json::Value::Null;

				#(#query)*

				Ok(state)
			}

            async fn execute(&self, command: Command) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
                let value = match command {
                    #(#execute)*
                };

            	Ok(value)
            }
		}
    }
    .into()
}
