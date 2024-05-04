#![allow(incomplete_features)]
#![feature(specialization)]
#![feature(unsize)]

use std::marker::Unsize;

pub trait Cast<P: ?Sized> {
    fn cast(&self) -> Option<&P>;
    fn cast_mut(&mut self) -> Option<&mut P>;
}

impl<D, P> Cast<P> for D
where
    P: ?Sized,
{
    default fn cast(&self) -> Option<&P> {
        None
    }

    default fn cast_mut(&mut self) -> Option<&mut P> {
        None
    }
}

impl<D, P> Cast<P> for D
where
    D: Unsize<P>,
    P: ?Sized,
{
    fn cast(&self) -> Option<&P> {
        Some(self)
    }

    fn cast_mut(&mut self) -> Option<&mut P> {
        Some(self)
    }
}
