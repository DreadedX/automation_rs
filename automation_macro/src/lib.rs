use proc_macro2::TokenStream;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::{parse_macro_input, DeriveInput, Token};

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

#[derive(Debug)]
enum Arg {
    Flatten,
    UserData,
    Rename(String),
    With(TokenStream),
    Default(Option<syn::Ident>),
}

impl syn::parse::Parse for Arg {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let arg = match input.parse::<syn::Ident>()?.to_string().as_str() {
            "flatten" => Arg::Flatten,
            "user_data" => Arg::UserData,
            "rename" => {
                input.parse::<Token![=]>()?;
                let lit = input.parse::<syn::Lit>()?;
                if let syn::Lit::Str(lit_str) = lit {
                    Arg::Rename(lit_str.value())
                } else {
                    panic!("Expected literal string");
                }
            }
            "with" => {
                input.parse::<Token![=]>()?;
                let lit = input.parse::<syn::Lit>()?;
                if let syn::Lit::Str(lit_str) = lit {
                    let token_stream: TokenStream = lit_str.parse()?;
                    Arg::With(token_stream)
                } else {
                    panic!("Expected literal string");
                }
            }
            "default" => {
                if input.parse::<Token![=]>().is_ok() {
                    let func = input.parse::<syn::Ident>()?;
                    Arg::Default(Some(func))
                } else {
                    Arg::Default(None)
                }
            }
            name => todo!("Handle unknown arg: {name}"),
        };

        Ok(arg)
    }
}

#[derive(Debug)]
struct ArgsParser {
    args: Punctuated<Arg, Token![,]>,
}

impl syn::parse::Parse for ArgsParser {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let args = input.parse_terminated(Arg::parse, Token![,])?;

        Ok(Self { args })
    }
}

#[derive(Debug)]
struct Args {
    flatten: bool,
    user_data: bool,
    rename: Option<String>,
    with: Option<TokenStream>,
    default: Option<Option<syn::Ident>>,
}

impl Args {
    fn new(args: Vec<Arg>) -> Self {
        let mut result = Args {
            flatten: false,
            user_data: false,
            rename: None,
            with: None,
            default: None,
        };
        for arg in args {
            match arg {
                Arg::Flatten => {
                    if result.flatten {
                        panic!("Option 'flatten' is already set")
                    }
                    result.flatten = true
                }
                Arg::UserData => {
                    if result.flatten {
                        panic!("Option 'user_data' is already set")
                    }
                    result.user_data = true
                }
                Arg::Rename(name) => {
                    if result.rename.is_some() {
                        panic!("Option 'rename' is already set")
                    }
                    result.rename = Some(name)
                }
                Arg::With(ty) => {
                    if result.with.is_some() {
                        panic!("Option 'with' is already set")
                    }
                    result.with = Some(ty)
                }
                Arg::Default(func) => {
                    if result.default.is_some() {
                        panic!("Option 'default' is already set")
                    }
                    result.default = Some(func)
                }
            }
        }

        if result.flatten && result.user_data {
            panic!("The options 'flatten' and 'user_data' conflict with each other")
        }

        if result.flatten && result.default.is_some() {
            panic!("The options 'flatten' and 'default' conflict with each other")
        }

        result
    }
}

#[proc_macro_derive(LuaDeviceConfig, attributes(device_config))]
pub fn lua_device_config_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);

    impl_lua_device_config_macro(&ast).into()
}

// struct Args

fn impl_lua_device_config_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    // TODO: Handle errors properly
    // This includes making sure one, and only one config is specified
    let fields = if let syn::Data::Struct(syn::DataStruct {
        fields: syn::Fields::Named(syn::FieldsNamed { ref named, .. }),
        ..
    }) = ast.data
    {
        named
    } else {
        unimplemented!("Macro can only handle named structs");
    };

    let fields: Vec<_> = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.clone().unwrap();
            let args: Vec<_> = field
                .attrs
                .iter()
                .filter_map(|attr| {
                    if attr.path().is_ident("device_config") {
                        let args: ArgsParser = attr.parse_args().unwrap();
                        Some(args.args)
                    } else {
                        None
                    }
                })
                .flatten()
                .collect();

            let args = Args::new(args);

            let table_name = if let Some(name) = args.rename {
                name
            } else {
                field_name.to_string()
            };

			// TODO: Improve how optional fields are detected
			let optional = if let syn::Type::Path(path) = field.ty.clone() {
				path.path.segments.first().unwrap().ident == "Option"
			} else {
				false
			};

            let default = if optional {
				quote! { None }
			} else if let Some(func) = args.default {
				if func.is_some() {
					quote! { #func() }
				} else {
					quote! { Default::default() }
				}
            } else {
				let missing = format!("Missing field '{table_name}'");
                quote! { panic!(#missing) }
            };

			let value = if args.flatten {
            	// println!("ValueFlatten: {}", field_name);
            	quote! {
            		mlua::LuaSerdeExt::from_value_with(lua, value.clone(), mlua::DeserializeOptions::new().deny_unsupported_types(false))?
            	}
			} else if args.user_data {
            	// println!("UserData: {}", field_name);
            	quote! {
            		if table.contains_key(#table_name)? {
						table.get(#table_name)?
            		} else {
						#default
					}
            	}
			} else {
            	// println!("Value: {}", field_name);
                quote! {
					{
						let #field_name: mlua::Value = table.get(#table_name)?;
						if !#field_name.is_nil() {
							mlua::LuaSerdeExt::from_value(lua, #field_name)?
						} else {
							#default
						}
					}
                }
			};

			let value = if let Some(temp_type) = args.with {
				if optional {
					quote! {
						{
							let temp: #temp_type = #value;
							temp.map(|v| v.into())
						}
					}
				} else {
					quote! {
						{
							let temp: #temp_type = #value;
							temp.into()
						}
					}
				}
			} else {
				value
			};

			quote! {
				#field_name: #value
			}
        })
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
