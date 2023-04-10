pub extern crate paste;

#[macro_export]
macro_rules! impl_setup {
    () => {
        pub trait As<T: ?Sized> {
            fn consume(self: Box<Self>) -> Option<Box<T>>;
            fn cast(&self) -> Option<&T>;
            fn cast_mut(&mut self) -> Option<&mut T>;
        }
    };
}

#[macro_export]
macro_rules! impl_cast {
    ($base:ident, $trait:ident) => {
        $crate::paste::paste! {
            impl<T: $base + $trait> As<dyn $trait> for T {
                fn consume(self: Box<Self>) -> Option<Box<dyn $trait>> {
                    Some(self)
                }

                fn cast(&self) -> Option<&dyn $trait> {
                    Some(self)
                }

                fn cast_mut(&mut  self) -> Option<&mut dyn $trait> {
                    Some(self)
                }
            }

            impl<T: $base> As<dyn $trait> for T {
                default fn consume(self: Box<Self>) -> Option<Box<dyn $trait>> {
                    None
                }

                default fn cast(&self) -> Option<&dyn $trait> {
                    None
                }

                default fn cast_mut(&mut self) -> Option<&mut dyn $trait> {
                    None
                }
            }
        }
    };
}
