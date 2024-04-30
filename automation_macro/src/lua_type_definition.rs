use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    AngleBracketedGenericArguments, Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed,
    PathArguments, Type, TypePath,
};

use crate::lua_device_config::{Args, Argument};

fn field_definition(field: &Field) -> TokenStream {
    let (args, _): (Vec<_>, Vec<_>) = field
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
    let args: Vec<_> = args.into_iter().flatten().collect();

    let field_name = if let Some(field_name) = args.iter().find_map(|arg| match arg {
        Argument::Rename { ident, .. } => Some(ident),
        _ => None,
    }) {
        field_name.value()
    } else {
        format!("{}", field.ident.clone().unwrap())
    };

    let mut optional = args
        .iter()
        .filter(|arg| matches!(arg, Argument::Default { .. } | Argument::DefaultExpr { .. }))
        .count()
        >= 1;

    if args
        .iter()
        .filter(|arg| matches!(arg, Argument::Flatten { .. }))
        .count()
        >= 1
    {
        let field_type = &field.ty;
        quote! {
            #field_type::generate_lua_fields().as_str()
        }
    } else {
        let path = if let Some(ty) = args.iter().find_map(|arg| match arg {
            Argument::From { ty, .. } => Some(ty),
            _ => None,
        }) {
            if let Type::Path(TypePath { path, .. }) = ty {
                path.clone()
            } else {
                todo!();
            }
        } else if let Type::Path(TypePath { path, .. }) = field.ty.clone() {
            path
        } else {
            todo!()
        };

        let seg = path.segments.first().unwrap();
        let field_type = if seg.ident == "Option" {
            if let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) =
                seg.arguments.clone()
            {
                optional = true;
                quote! { stringify!(#args) }
            } else {
                unreachable!("Option should always have angle brackets");
            }
        } else if seg.ident == "Vec" {
            if let PathArguments::AngleBracketed(AngleBracketedGenericArguments { args, .. }) =
                seg.arguments.clone()
            {
                optional = true;
                quote! { stringify!(#args[]) }
            } else {
                unreachable!("Option should always have angle brackets");
            }
        } else {
            quote! { stringify!(#path).replace(" :: ", "_") }
        };

        let mut format = "--- @field {} {}".to_string();
        if optional {
            format += "|nil";
        }
        format += "\n";

        quote! {
            format!(#format, #field_name, #field_type).as_str()
        }
    }
}

pub fn impl_lua_type_definition(ast: &DeriveInput) -> TokenStream {
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

    let fields: Vec<_> = fields.iter().map(field_definition).collect();

    let gen = quote! {
        impl #name {
            pub fn generate_lua_definition() -> String {
                let mut def = format!("--- @class {}\n", stringify!(#name));

                def += #name::generate_lua_fields().as_str();

                def
            }

            pub fn generate_lua_fields() -> String {
                let mut def = String::new();

                #(def += #fields;)*

                def
            }
        }
    };

    gen
}
