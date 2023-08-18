use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use impl_cast::device_trait;

#[async_trait]
#[device_trait]
pub trait Timeout {
    async fn start_timeout(&mut self, _timeout: Duration) -> Result<()>;
    async fn stop_timeout(&mut self) -> Result<()>;
}
