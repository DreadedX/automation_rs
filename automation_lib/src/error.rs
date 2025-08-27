use std::{error, fmt, result};

use bytes::Bytes;
use rumqttc::ClientError;
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
            unreachable!(
                "This error should only be returned if there are actually missing environment variables"
            );
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
