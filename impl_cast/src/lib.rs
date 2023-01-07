pub extern crate paste;

#[macro_export]
macro_rules! impl_cast {
    ($base:ident, $trait:ident) => {
        $crate::paste::paste! {
            pub trait [< As $trait>] {
                fn consume(self: Box<Self>) -> Option<Box<dyn $trait + Sync + Send>>;
                fn cast(&self) -> Option<&dyn $trait>;
                fn cast_mut(&mut self) -> Option<&mut dyn $trait>;
            }

            impl<T: $base> [< As $trait>] for T {
                default fn consume(self: Box<Self>) -> Option<Box<dyn $trait + Sync + Send>> {
                    None
                }
                default fn cast(&self) -> Option<&dyn $trait> {
                    None
                }
                default fn cast_mut(&mut self) -> Option<&mut dyn $trait> {
                    None
                }
            }

            impl<T: $base + $trait + Sync + Send + 'static> [< As $trait>] for T {
                fn consume(self: Box<Self>) -> Option<Box<dyn $trait + Sync + Send>> {
                    Some(self)
                }
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
