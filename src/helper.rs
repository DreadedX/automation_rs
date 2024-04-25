use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DurationSeconds(u64);

impl From<DurationSeconds> for Duration {
    fn from(value: DurationSeconds) -> Self {
        Self::from_secs(value.0)
    }
}

#[derive(Debug, Deserialize)]
pub struct Ipv4SocketAddr<const PORT: u16>(Ipv4Addr);

impl<const PORT: u16> From<Ipv4SocketAddr<PORT>> for SocketAddr {
    fn from(ip: Ipv4SocketAddr<PORT>) -> Self {
        Self::from((ip.0, PORT))
    }
}
