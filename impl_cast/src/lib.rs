use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{parse::Parse, parse_macro_input, Ident, ItemTrait, Path, Token, TypeParamBound};

struct Attr {
    name: Ident,
    traits: Vec<Path>,
}

impl Parse for Attr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut traits = Vec::new();

        let name = input.parse::<Ident>()?;
        input.parse::<Token![:]>()?;

        loop {
            let ty = input.parse()?;
            traits.push(ty);

            if input.is_empty() {
                break;
            }

            input.parse::<Token![+]>()?;
        }

        Ok(Attr { name, traits })
    }
}

/// This macro enables optional trait bounds on a trait with an appropriate cast trait to convert
/// to the optional traits
/// # Example
///
/// ```
/// #![feature(specialization)]
///
/// // Create some traits
/// #[impl_cast::device_trait]
/// trait OnOff {}
/// #[impl_cast::device_trait]
/// trait Brightness {}
///
/// // Create the main device trait
/// #[impl_cast::device(As: OnOff + Brightness)]
/// trait Device {}
///
/// // Create an implementation
/// struct ExampleDevice {}
/// impl Device for ExampleDevice {}
/// impl OnOff for ExampleDevice {}
///
/// // Creates a boxed instance of the example device
/// let example_device: Box<dyn Device> = Box::new(ExampleDevice {});
///
/// // Cast to the OnOff trait, which is implemented
/// let as_on_off = As::<dyn OnOff>::cast(example_device.as_ref());
/// assert!(as_on_off.is_some());
///
/// // Cast to the Brightness trait, which is not implemented
/// let as_on_off = As::<dyn Brightness>::cast(example_device.as_ref());
/// assert!(as_on_off.is_none());
///
/// // Finally we are going to consume the example device into an instance of the OnOff trait
/// let consumed = As::<dyn OnOff>::consume(example_device);
/// assert!(consumed.is_some())
/// ```
#[proc_macro_attribute]
pub fn device(attr: TokenStream, item: TokenStream) -> TokenStream {
    let Attr { name, traits } = parse_macro_input!(attr);
    let mut interface: ItemTrait = parse_macro_input!(item);

    let prefix = quote! {
        pub trait #name<T: ?Sized + 'static> {
            fn consume(self: Box<Self>) -> Option<Box<T>>;
            fn cast(&self) -> Option<&T>;
            fn cast_mut(&mut self) -> Option<&mut T>;
        }
    };

    traits.iter().for_each(|device_trait| {
        interface.supertraits.push(TypeParamBound::Verbatim(quote! {
            #name<dyn #device_trait>
        }));
    });

    let interface_ident = format_ident!("{}", interface.ident);
    let impls = traits
        .iter()
        .map(|device_trait| {
            quote! {
                // Default impl
                impl<T> #name<dyn #device_trait> for T
                where
                    T: #interface_ident + 'static,
                {
                    default fn consume(self: Box<Self>) -> Option<Box<dyn #device_trait>> {
                        None
                    }

                    default fn cast(&self) -> Option<&(dyn #device_trait + 'static)> {
                        None
                    }

                    default fn cast_mut(&mut self) -> Option<&mut (dyn #device_trait + 'static)> {
                        None
                    }
                }

                // Specialization, should not cause any unsoundness as we dispatch based on
                // #device_trait
                impl<T> #name<dyn #device_trait> for T
                where
                    T: #interface_ident + #device_trait + 'static,
                {
                    fn consume(self: Box<Self>) -> Option<Box<dyn #device_trait>> {
                        Some(self)
                    }

                    fn cast(&self) -> Option<&(dyn #device_trait + 'static)> {
                        Some(self)
                    }

                    fn cast_mut(&mut  self) -> Option<&mut (dyn #device_trait + 'static)> {
                        Some(self)
                    }
                }
            }
        })
        .fold(quote! {}, |acc, x| {
            quote! {
                // Not sure if this is the right way to do this
                #acc
                #x
            }
        });

    let tokens = quote! {
        #interface
        #prefix
        #impls
    };

    tokens.into()
}

// TODO: Not sure if this makes sense to have?
/// This macro ensures that the device traits have the correct trait bounds
#[proc_macro_attribute]
pub fn device_trait(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut interface: ItemTrait = parse_macro_input!(item);

    interface.supertraits.push(TypeParamBound::Verbatim(quote! {
        ::core::marker::Sync + ::core::marker::Send
    }));

    #[cfg(feature = "debug")]
    interface.supertraits.push(TypeParamBound::Verbatim(quote! {
        ::std::fmt::Debug
    }));

    interface.into_token_stream().into()
}
