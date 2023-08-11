#![allow(incomplete_features)]
#![feature(specialization)]
#![feature(let_chains)]
pub mod device;
mod fullfillment;

mod request;
mod response;

mod attributes;
pub mod errors;
pub mod traits;
pub mod types;

pub use device::GoogleHomeDevice;
pub use fullfillment::FullfillmentError;
pub use fullfillment::GoogleHome;
pub use request::Request;
pub use response::Response;
