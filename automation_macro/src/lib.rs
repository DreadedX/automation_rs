use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Paren;
use syn::{parenthesized, parse_macro_input, DeriveInput, Expr, LitStr, Result, Token};

#[proc_macro_derive(LuaDevice, attributes(config))]
pub fn lua_device_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    impl_lua_device_macro(&ast).into()
}

fn impl_lua_device_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
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

mod kw {
    syn::custom_keyword!(device_config);
    syn::custom_keyword!(flatten);
    syn::custom_keyword!(from_lua);
    syn::custom_keyword!(rename);
    syn::custom_keyword!(with);
    syn::custom_keyword!(from);
    syn::custom_keyword!(default);
}

#[derive(Debug)]
enum Argument {
    Flatten {
        _keyword: kw::flatten,
    },
    FromLua {
        _keyword: kw::from_lua,
    },
    Rename {
        _keyword: kw::rename,
        _paren: Paren,
        ident: LitStr,
    },
    With {
        _keyword: kw::with,
        _paren: Paren,
        // TODO: Ideally we capture this better
        expr: Expr,
    },
    From {
        _keyword: kw::from,
        _paren: Paren,
        ty: syn::Type,
    },
    Default {
        _keyword: kw::default,
    },
    DefaultExpr {
        _keyword: kw::default,
        _paren: Paren,
        expr: Expr,
    },
}

impl Parse for Argument {
    fn parse(input: ParseStream) -> Result<Self> {
        let lookahead = input.lookahead1();
        if lookahead.peek(kw::flatten) {
            Ok(Self::Flatten {
                _keyword: input.parse()?,
            })
        } else if lookahead.peek(kw::from_lua) {
            Ok(Self::FromLua {
                _keyword: input.parse()?,
            })
        } else if lookahead.peek(kw::rename) {
            let content;
            Ok(Self::Rename {
                _keyword: input.parse()?,
                _paren: parenthesized!(content in input),
                ident: content.parse()?,
            })
        } else if lookahead.peek(kw::with) {
            let content;
            Ok(Self::With {
                _keyword: input.parse()?,
                _paren: parenthesized!(content in input),
                expr: content.parse()?,
            })
        } else if lookahead.peek(kw::from) {
            let content;
            Ok(Self::From {
                _keyword: input.parse()?,
                _paren: parenthesized!(content in input),
                ty: content.parse()?,
            })
        } else if lookahead.peek(kw::default) {
            let keyword = input.parse()?;
            if input.peek(Paren) {
                let content;
                Ok(Self::DefaultExpr {
                    _keyword: keyword,
                    _paren: parenthesized!(content in input),
                    expr: content.parse()?,
                })
            } else {
                Ok(Self::Default { _keyword: keyword })
            }
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Debug)]
struct Args {
    args: Punctuated<Argument, Token![,]>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        Ok(Self {
            args: input.parse_terminated(Argument::parse, Token![,])?,
        })
    }
}

#[proc_macro_derive(LuaDeviceConfig, attributes(device_config))]
pub fn lua_device_config_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    impl_lua_device_config_macro(&ast).into()
}

fn impl_lua_device_config_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = ast.data
    {
        named
    } else {
        return quote_spanned! {ast.span() => compile_error!("This macro only works on named structs")};
    };

    let field_names: Vec<_> = fields
        .iter()
        .map(|field| field.ident.clone().unwrap())
        .collect();

    let fields: Vec<_> = fields
		.iter()
		.map(|field| {
			let field_name = field.ident.clone().unwrap();
			let (args, errors): (Vec<_>, Vec<_>) = field
				.attrs
				.iter()
				.filter_map(|attr| {
					if attr.path().is_ident("device_config") {
						Some(attr.parse_args::<Args>().map(|args| args.args))
					} else {
						None
					}
				})
				.partition_result();

			let errors: Vec<_> = errors
				.iter()
				.map(|error| error.to_compile_error())
				.collect();

			if !errors.is_empty() {
				return quote! { #(#errors)* };
			}

			let args: Vec<_> = args.into_iter().flatten().collect();

			let table_name = match args
				.iter()
				.filter_map(|arg| match arg {
					Argument::Rename { ident, .. } => Some(ident.value()),
					_ => None,
				})
				.collect::<Vec<_>>()
			.as_slice()
			{
				[] => field_name.to_string(),
				[rename] => rename.to_owned(),
				_ => return quote_spanned! {field.span() => compile_error!("Field contains duplicate 'rename'")},
			};

			// TODO: Detect Option<_> properly and use Default::default() as fallback automatically
			let missing = format!("Missing field '{table_name}'");
			let default = match args
				.iter()
				.filter_map(|arg| match arg {
					Argument::Default { .. } => Some(quote! { Default::default() }),
					Argument::DefaultExpr { expr, .. } => Some(quote! { (#expr) }),
					_ => None,
				})
				.collect::<Vec<_>>()
			.as_slice()
			{
				[] => quote! {panic!(#missing)},
				[default] => default.to_owned(),
				_ => return quote_spanned! {field.span() => compile_error!("Field contains duplicate 'default'")},
			};


			let value = match args
				.iter()
				.filter_map(|arg| match arg {
					Argument::Flatten { .. } => Some(quote! {
						mlua::LuaSerdeExt::from_value_with(lua, value.clone(), mlua::DeserializeOptions::new().deny_unsupported_types(false))?
					}),
					Argument::FromLua { .. } => Some(quote! {
						if table.contains_key(#table_name)? {
							table.get(#table_name)?
						} else {
							#default
						}
					}),
					_ => None,
				})
				.collect::<Vec<_>>()
			.as_slice() {
				[] => quote! {
					{
						let #field_name: mlua::Value = table.get(#table_name)?;
						if !#field_name.is_nil() {
							mlua::LuaSerdeExt::from_value(lua, #field_name)?
						} else {
							#default
						}
					}
				},
				[value] => value.to_owned(),
				_ => return quote_spanned! {field.span() => compile_error!("Only one of either 'flatten' or 'from_lua' is allowed")},
			};

			let value = match args
				.iter()
				.filter_map(|arg| match arg {
					Argument::From { ty, .. } => Some(quote! {
						{
							let temp: #ty = #value;
							temp.into()
						}
					}),
					Argument::With { expr, .. } => Some(quote! {
						{
							let temp = #value;
							(#expr)(temp)
						}
					}),
					_ => None,
				})
				.collect::<Vec<_>>()
			.as_slice() {
				[] => value,
				[value] => value.to_owned(),
				_ => return quote_spanned! {field.span() => compile_error!("Field contains duplicate 'as'")},
			};

			quote! { #value }
		})
		.zip(field_names)
		.map(|(value, name)| quote! { #name: #value })
		.collect();

    let gen = quote! {
        impl<'lua> mlua::FromLua<'lua> for #name {
            fn from_lua(value: mlua::Value<'lua>, lua: &'lua mlua::Lua) -> mlua::Result<Self> {
                if !value.is_table() {
                    panic!("Expected table");
                }
                let table = value.as_table().unwrap();

                Ok(#name {
                    #(#fields,)*
            })

            }
        }
    };

    gen
}
