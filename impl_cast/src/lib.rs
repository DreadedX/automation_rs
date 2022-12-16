pub extern crate paste;

#[macro_export]
macro_rules! impl_cast {
    ($base:ident, $trait:ident) => {
        $crate::paste::paste! {
            pub trait [< As $trait>] {
                fn cast(&self) -> Option<&dyn $trait>;
                fn cast_mut(&mut self) -> Option<&mut dyn $trait>;
            }

            impl<T: $base> [< As $trait>] for T {
                default fn cast(&self) -> Option<&dyn $trait> {
                    None
                }
                default fn cast_mut(&mut self) -> Option<&mut dyn $trait> {
                    None
                }
            }

            impl<T: $base + $trait> [< As $trait>] for T {
                fn cast(&self) -> Option<&dyn $trait> {
                    Some(self)
                }
                fn cast_mut(&mut self) -> Option<&mut dyn $trait> {
                    Some(self)
                }
            }
        }
    };
}
