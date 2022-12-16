use serde::Serialize;

use crate::{device::GoogleHomeDevice, errors::ErrorCode};

#[derive(Debug, Serialize)]
pub enum Trait {
    #[serde(rename = "action.devices.traits.OnOff")]
    OnOff,
    #[serde(rename = "action.devices.traits.Scene")]
    Scene,
}

pub trait OnOff {
    fn is_command_only(&self) -> Option<bool> {
        None
    }

    fn is_query_only(&self) -> Option<bool> {
        None
    }

    // @TODO Implement correct error so we can handle them properly
    fn is_on(&self) -> Result<bool, ErrorCode>;
    fn set_on(&mut self, on: bool) -> Result<(), ErrorCode>;
}
impl_cast::impl_cast!(GoogleHomeDevice, OnOff);

pub trait Scene {
    fn is_scene_reversible(&self) -> Option<bool> {
        None
    }

    fn set_active(&self, activate: bool) -> Result<(), ErrorCode>;
}
impl_cast::impl_cast!(GoogleHomeDevice, Scene);
