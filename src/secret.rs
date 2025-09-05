use std::str::from_utf8;

use config::{ConfigError, Source, Value, ValueKind};

#[derive(Debug, Clone, Default)]
pub struct EnvironmentSecretFile {}

const SUFFIX: &str = "__file";
const PREFIX: &str = concat!(std::env!("CARGO_PKG_NAME"), "__");

impl Source for EnvironmentSecretFile {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new((*self).clone())
    }

    fn collect(&self) -> Result<config::Map<String, config::Value>, ConfigError> {
        Ok(std::env::vars()
            .flat_map(|(key, value): (String, String)| {
                let key = key.to_lowercase();
                if !key.starts_with(PREFIX) {
                    return None;
                }

                if !key.ends_with(SUFFIX) {
                    return None;
                }

                let suffix_length = key.len() - SUFFIX.len();
                let key = key[PREFIX.len()..suffix_length].replace("__", ".");

                if key.is_empty() {
                    return None;
                }

                let content = from_utf8(&std::fs::read(&value).unwrap())
                    .unwrap()
                    .to_owned();

                Some((key, Value::new(Some(&value), ValueKind::String(content))))
            })
            .collect())
    }
}
