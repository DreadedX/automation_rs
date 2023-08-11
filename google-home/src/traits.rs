use async_trait::async_trait;
use serde::Serialize;

use crate::errors::ErrorCode;

#[derive(Debug, Serialize)]
pub enum Trait {
    #[serde(rename = "action.devices.traits.OnOff")]
    OnOff,
    #[serde(rename = "action.devices.traits.Scene")]
    Scene,
}

#[async_trait]
#[impl_cast::device_trait]
pub trait OnOff {
    fn is_command_only(&self) -> Option<bool> {
        None
    }

    fn is_query_only(&self) -> Option<bool> {
        None
    }

    // TODO: Implement correct error so we can handle them properly
    async fn is_on(&self) -> Result<bool, ErrorCode>;
    async fn set_on(&mut self, on: bool) -> Result<(), ErrorCode>;
}

#[async_trait]
#[impl_cast::device_trait]
pub trait Scene {
    fn is_scene_reversible(&self) -> Option<bool> {
        None
    }

    async fn set_active(&self, activate: bool) -> Result<(), ErrorCode>;
}
