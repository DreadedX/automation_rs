use std::{error, fmt, result};

use axum::http::status::InvalidStatusCode;
use axum::response::IntoResponse;
use bytes::Bytes;
use rumqttc::ClientError;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct MissingEnv {
    keys: Vec<String>,
}

// TODO: Would be nice to somehow get the line number of the missing keys
impl MissingEnv {
    pub fn new() -> Self {
        Self { keys: Vec::new() }
    }

    pub fn add_missing(&mut self, key: &str) {
        self.keys.push(key.into());
    }

    pub fn has_missing(self) -> result::Result<(), Self> {
        if !self.keys.is_empty() {
            Err(self)
        } else {
            Ok(())
        }
    }
}

impl Default for MissingEnv {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MissingEnv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Missing environment variable")?;
        if self.keys.is_empty() {
            unreachable!("This error should only be returned if there are actually missing environment variables");
        }
        if self.keys.len() == 1 {
            write!(f, " '{}'", self.keys[0])?;
        } else {
            write!(f, "s '{}'", self.keys[0])?;
            self.keys
                .iter()
                .skip(1)
                .try_for_each(|key| write!(f, ", '{key}'"))?;
        }

        Ok(())
    }
}

impl error::Error for MissingEnv {}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Invalid message payload received: {0:?}")]
    InvalidPayload(Bytes),
}

// TODO: Would be nice to somehow get the line number of the expected wildcard topic
#[derive(Debug, Error)]
#[error("Topic '{topic}' is expected to be a wildcard topic")]
pub struct MissingWildcard {
    topic: String,
}

impl MissingWildcard {
    pub fn new(topic: &str) -> Self {
        Self {
            topic: topic.into(),
        }
    }
}

#[derive(Debug, Error)]
pub enum DeviceConfigError {
    #[error("Device '{0}' does not implement expected trait '{1}'")]
    MissingTrait(String, String),
    #[error(transparent)]
    MqttClientError(#[from] rumqttc::ClientError),
}

#[derive(Debug, Error)]
pub enum PresenceError {
    #[error(transparent)]
    SubscribeError(#[from] ClientError),
    #[error(transparent)]
    MissingWildcard(#[from] MissingWildcard),
}

#[derive(Debug, Error)]
pub enum LightSensorError {
    #[error(transparent)]
    SubscribeError(#[from] ClientError),
}

#[derive(Debug, Error)]
#[error("{source}")]
pub struct ApiError {
    status_code: axum::http::StatusCode,
    source: Box<dyn std::error::Error>,
}

impl ApiError {
    pub fn new(status_code: axum::http::StatusCode, source: Box<dyn std::error::Error>) -> Self {
        Self {
            status_code,
            source,
        }
    }
}

impl From<ApiError> for ApiErrorJson {
    fn from(value: ApiError) -> Self {
        let error = ApiErrorJsonError {
            code: value.status_code.as_u16(),
            status: value.status_code.to_string(),
            reason: value.source.to_string(),
        };

        Self { error }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            self.status_code,
            serde_json::to_string::<ApiErrorJson>(&self.into())
                .expect("Serialization should not fail"),
        )
            .into_response()
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiErrorJsonError {
    code: u16,
    status: String,
    reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiErrorJson {
    error: ApiErrorJsonError,
}

impl TryFrom<ApiErrorJson> for ApiError {
    type Error = InvalidStatusCode;

    fn try_from(value: ApiErrorJson) -> result::Result<Self, Self::Error> {
        let status_code = axum::http::StatusCode::from_u16(value.error.code)?;
        let source = value.error.reason.into();

        Ok(Self {
            status_code,
            source,
        })
    }
}
