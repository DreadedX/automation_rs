#![allow(incomplete_features)]
#![feature(specialization)]
#![feature(let_chains)]
pub mod device;
mod fulfillment;

mod request;
mod response;

mod attributes;
pub mod errors;
pub mod traits;
pub mod types;

pub use device::GoogleHomeDevice;
pub use fulfillment::{FulfillmentError, GoogleHome};
pub use request::Request;
pub use response::Response;
