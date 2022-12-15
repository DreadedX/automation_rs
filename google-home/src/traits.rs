use serde::Serialize;

use crate::device::GoogleHomeDevice;

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
    fn is_on(&self) -> Result<bool, anyhow::Error>;
    fn set_on(&mut self, on: bool) -> Result<(), anyhow::Error>;
}
pub trait AsOnOff {
    fn cast(&self) -> Option<&dyn OnOff> {
        None
    }
    fn cast_mut(&mut self) -> Option<&mut dyn OnOff> {
        None
    }
}
impl<'a, T: GoogleHomeDevice<'a> + OnOff> AsOnOff for T {
    fn cast(&self) -> Option<&dyn OnOff> {
        Some(self)
    }
    fn cast_mut(&mut self) -> Option<&mut dyn OnOff> {
        Some(self)
    }
}


pub trait Scene {
    fn is_scene_reversible(&self) -> Option<bool> {
        None
    }

    fn set_active(&self, activate: bool) -> Result<(), anyhow::Error>;
}
pub trait AsScene {
    fn cast(&self) -> Option<&dyn Scene> {
        None
    }
    fn cast_mut(&mut self) -> Option<&mut dyn Scene> {
        None
    }
}
impl<'a, T: GoogleHomeDevice<'a> + Scene> AsScene for T {
    fn cast(&self) -> Option<&dyn Scene> {
        Some(self)
    }
    fn cast_mut(&mut self) -> Option<&mut dyn Scene> {
        Some(self)
    }
}
