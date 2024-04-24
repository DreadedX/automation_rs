mod air_filter;
mod audio_setup;
mod contact_sensor;
mod debug_bridge;
mod hue_bridge;
mod hue_light;
mod ikea_outlet;
mod kasa_outlet;
mod light_sensor;
mod ntfy;
mod presence;
mod wake_on_lan;
mod washer;

use std::fmt::Debug;

use automation_cast::Cast;
use google_home::traits::OnOff;
use google_home::GoogleHomeDevice;

pub use self::air_filter::*;
pub use self::audio_setup::*;
pub use self::contact_sensor::*;
pub use self::debug_bridge::*;
pub use self::hue_bridge::*;
pub use self::hue_light::*;
pub use self::ikea_outlet::*;
pub use self::kasa_outlet::*;
pub use self::light_sensor::{LightSensor, LightSensorConfig};
pub use self::ntfy::{Notification, Ntfy};
pub use self::presence::{Presence, PresenceConfig, DEFAULT_PRESENCE};
pub use self::wake_on_lan::*;
pub use self::washer::*;
use crate::event::{OnDarkness, OnMqtt, OnNotification, OnPresence};
use crate::traits::Timeout;

pub trait Device:
    Debug
    + Sync
    + Send
    + Cast<dyn GoogleHomeDevice>
    + Cast<dyn OnMqtt>
    + Cast<dyn OnMqtt>
    + Cast<dyn OnPresence>
    + Cast<dyn OnDarkness>
    + Cast<dyn OnNotification>
    + Cast<dyn OnOff>
    + Cast<dyn Timeout>
{
    fn get_id(&self) -> &str;
}
