use std::time::Duration;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DurationSeconds(u64);

impl From<DurationSeconds> for Duration {
    fn from(value: DurationSeconds) -> Self {
        Self::from_secs(value.0)
    }
}
