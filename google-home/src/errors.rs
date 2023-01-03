use thiserror::Error;
use serde::Serialize;

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone, Serialize, Error)]
#[serde(rename_all = "camelCase")]
pub enum DeviceError {
    #[error("deviceNotFound")]
    DeviceNotFound,
    #[error("deviceOffline")]
    DeviceOffline,
    #[error("actionNotAvailable")]
    ActionNotAvailable,
    #[error("transientError")]
    TransientError,
}

#[derive(Debug, Hash, PartialEq, Eq, Copy, Clone, Serialize, Error)]
#[serde(rename_all = "camelCase")]
pub enum DeviceException {
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy, Serialize, Error)]
#[serde(untagged)]
pub enum ErrorCode {
    #[error("{0}")]
    DeviceError(DeviceError),
    #[error("{0}")]
    DeviceException(DeviceException),
}

impl From<DeviceError> for ErrorCode {
    fn from(value: DeviceError) -> Self {
        Self::DeviceError(value)
    }
}

impl From<DeviceException> for ErrorCode {
    fn from(value: DeviceException) -> Self {
        Self::DeviceException(value)
    }
}
