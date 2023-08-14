use std::time::Duration;

use impl_cast::device_trait;

#[device_trait]
pub trait Timeout {
    fn start_timeout(&mut self, _timeout: Duration) {}
}
