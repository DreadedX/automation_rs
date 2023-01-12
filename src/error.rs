use std::{fmt, error, result};

use axum::{response::IntoResponse, http::status::InvalidStatusCode};
use serde::{Serialize, Deserialize};

pub type Error = Box<dyn error::Error>;
pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct MissingEnv {
    keys: Vec<String>
}

// @TODO Would be nice to somehow get the line number of the missing keys
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


// @TODO Would be nice to somehow get the line number of the expected wildcard topic
#[derive(Debug, Clone)]
pub struct MissingWildcard {
    topic: String
}

impl MissingWildcard {
    pub fn new(topic: &str) -> Self {
        Self { topic: topic.to_owned() }
    }
}

impl fmt::Display for MissingWildcard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Topic '{}' is exptected to be a wildcard topic", self.topic)
    }
}

impl error::Error for MissingWildcard {}


#[derive(Debug)]
pub struct FailedToParseConfig {
    config: String,
    cause: Error,
}

impl FailedToParseConfig {
    pub fn new(config: &str, cause: Error) -> Self {
        Self { config: config.to_owned(), cause }
    }
}

impl fmt::Display for FailedToParseConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to parse config '{}'", self.config)
    }
}

impl error::Error for FailedToParseConfig {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(self.cause.as_ref())
    }
}


#[derive(Debug)]
pub struct FailedToCreateDevice {
    device: String,
    cause: Error,
}

impl FailedToCreateDevice {
    pub fn new(device: &str, cause: Error) -> Self {
        Self { device: device.to_owned(), cause }
    }
}

impl fmt::Display for FailedToCreateDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Failed to create device '{}'", self.device)
    }
}

impl error::Error for FailedToCreateDevice {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(self.cause.as_ref())
    }
}


#[derive(Debug, Clone)]
pub struct ExpectedOnOff {
    device: String
}

impl ExpectedOnOff {
    pub fn new(device: &str) -> Self {
        Self { device: device.to_owned() }
    }
}

impl fmt::Display for ExpectedOnOff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Expected device '{}' to implement OnOff trait", self.device)
    }
}

impl error::Error for ExpectedOnOff {}


#[derive(Debug)]
pub struct ApiError {
    status_code: axum::http::StatusCode,
    error: Error,
}

impl ApiError {
    pub fn new(status_code: axum::http::StatusCode, error: Error) -> Self {
        Self { status_code, error }
    }

    pub fn prepare_for_json(&self) -> ApiErrorJson {
        let error = ApiErrorJsonError {
            code: self.status_code.as_u16(),
            status: self.status_code.to_string(),
            reason: self.error.to_string(),
        };

        ApiErrorJson { error }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.error.fmt(f)
    }
}

impl error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.status_code, serde_json::to_string(&self.prepare_for_json()).unwrap()).into_response()
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
        let error = value.error.reason.into();

        Ok(Self { status_code, error })
    }
}
