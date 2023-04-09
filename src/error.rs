use std::{fmt, error, result};

use rumqttc::ClientError;
use thiserror::Error;
use axum::{response::IntoResponse, http::status::InvalidStatusCode};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone)]
pub struct MissingEnv {
    keys: Vec<String>
}

// TODO: Would be nice to somehow get the line number of the missing keys
impl MissingEnv {
    pub fn new() -> Self {
        Self { keys: Vec::new() }
    }

    pub fn add_missing(&mut self, key: &str) {
        self.keys.push(key.to_owned());
    }

    pub fn has_missing(self) -> result::Result<(), Self> {
        if self.keys.len() > 0 {
            Err(self)
        } else {
            Ok(())
        }
    }
}

impl fmt::Display for MissingEnv {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Missing environment variable")?;
        if self.keys.len() == 0 {
            unreachable!("This error should only be returned if there are actually missing environment variables");
        }
        if self.keys.len() == 1 {
            write!(f, " '{}'", self.keys[0])?;
        } else {
            write!(f, "s '{}'", self.keys[0])?;
            self.keys.iter().skip(1).map(|key| {
                write!(f, ", '{key}'")
            }).collect::<fmt::Result>()?;
        }

        Ok(())
    }
}

impl error::Error for MissingEnv {}

#[derive(Debug, Error)]
pub enum ConfigParseError {
    #[error(transparent)]
    MissingEnv(#[from] MissingEnv),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    DeserializeError(#[from] toml::de::Error)
}

// TODO: Would be nice to somehow get the line number of the expected wildcard topic
#[derive(Debug, Error)]
#[error("Topic '{topic}' is expected to be a wildcard topic")]
pub struct MissingWildcard {
    topic: String
}

impl MissingWildcard {
    pub fn new(topic: &str) -> Self {
        Self { topic: topic.to_owned() }
    }
}

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error(transparent)]
    SubscribeError(#[from] ClientError),
    #[error("Expected device '{0}' to implement OnOff trait")]
    OnOffExpected(String)
}

#[derive(Debug, Error)]
pub enum DeviceCreationError {
    #[error(transparent)]
    DeviceError(#[from] DeviceError),
    #[error(transparent)]
    MissingWildcard(#[from] MissingWildcard),
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
        Self { status_code, source }
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
        (self.status_code, serde_json::to_string::<ApiErrorJson>(&self.into()).unwrap()).into_response()
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

        Ok(Self { status_code, source })
    }
}
