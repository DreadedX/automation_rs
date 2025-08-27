use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Paren;
use syn::{
    Data, DataStruct, DeriveInput, Expr, Field, Fields, FieldsNamed, LitStr, Result, Token, Type,
    parenthesized,
};

mod kw {
    use syn::custom_keyword;

    custom_keyword!(device_config);
    custom_keyword!(flatten);
    custom_keyword!(from_lua);
    custom_keyword!(rename);
    custom_keyword!(with);
    custom_keyword!(from);
    custom_keyword!(default);
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
        ty: Type,
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

fn field_from_lua(field: &Field) -> TokenStream {
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
        [] => field.ident.clone().unwrap().to_string(),
        [rename] => rename.to_owned(),
        _ => {
            return quote_spanned! {field.span() => compile_error!("Field contains duplicate 'rename'")};
        }
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
        _ => {
            return quote_spanned! {field.span() => compile_error!("Field contains duplicate 'default'")};
        }
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
				let value: mlua::Value = table.get(#table_name)?;
				if !value.is_nil() {
					mlua::LuaSerdeExt::from_value(lua, value)?
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
        .as_slice()
    {
        [] => value,
        [value] => value.to_owned(),
        _ => {
            return quote_spanned! {field.span() => compile_error!("Only one of either 'from' or 'with' is allowed")};
        }
    };

    quote! { #value }
}

pub fn impl_lua_device_config_macro(ast: &DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let fields = if let Data::Struct(DataStruct {
        fields: Fields::Named(FieldsNamed { ref named, .. }),
        ..
    }) = ast.data
    {
        named
    } else {
        return quote_spanned! {ast.span() => compile_error!("This macro only works on named structs")};
    };

    let lua_fields: Vec<_> = fields
        .iter()
        .map(|field| {
            let name = field.ident.clone().unwrap();
            let value = field_from_lua(field);
            quote! { #name: #value }
        })
        .collect();

    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();
    let impl_from_lua = quote! {
        impl #impl_generics mlua::FromLua for #name #type_generics #where_clause {
            fn from_lua(value: mlua::Value, lua: &mlua::Lua) -> mlua::Result<Self> {
                if !value.is_table() {
                    panic!("Expected table");
                }
                let table = value.as_table().unwrap();

                Ok(#name {
                    #(#lua_fields,)*
            })

            }
        }
    };

    impl_from_lua
}
