#![feature(specialization)]
mod fullfillment;
pub mod device;

mod request;
mod response;

pub mod types;
pub mod traits;
pub mod errors;
mod attributes;

pub use fullfillment::GoogleHome;
pub use device::Fullfillment;
pub use device::GoogleHomeDevice;
