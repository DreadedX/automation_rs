use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Timeout: Sync + Send {
    async fn start_timeout(&self, _timeout: Duration) -> Result<()>;
    async fn stop_timeout(&self) -> Result<()>;
}
