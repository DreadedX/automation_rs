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
pub use fullfillment::FullfillmentError;
pub use request::Request;
pub use response::Response;
pub use device::GoogleHomeDevice;
