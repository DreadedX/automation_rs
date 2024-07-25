#![allow(incomplete_features)]
#![feature(specialization)]
#![feature(unsize)]

use std::marker::Unsize;

pub trait Cast<P: ?Sized> {
    fn cast(&self) -> Option<&P>;
}

impl<D, P> Cast<P> for D
where
    P: ?Sized,
{
    default fn cast(&self) -> Option<&P> {
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
}
