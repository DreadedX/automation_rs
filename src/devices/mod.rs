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

use google_home::device::AsGoogleHomeDevice;
use google_home::traits::OnOff;

pub use self::air_filter::AirFilterConfig;
pub use self::audio_setup::AudioSetupConfig;
pub use self::contact_sensor::ContactSensorConfig;
pub use self::debug_bridge::DebugBridgeConfig;
pub use self::hue_bridge::HueBridgeConfig;
pub use self::hue_light::HueGroupConfig;
pub use self::ikea_outlet::IkeaOutletConfig;
pub use self::kasa_outlet::KasaOutletConfig;
pub use self::light_sensor::{LightSensor, LightSensorConfig};
pub use self::ntfy::{Notification, Ntfy};
pub use self::presence::{Presence, PresenceConfig, DEFAULT_PRESENCE};
pub use self::wake_on_lan::WakeOnLANConfig;
pub use self::washer::WasherConfig;
use crate::event::{OnDarkness, OnMqtt, OnNotification, OnPresence};
use crate::traits::Timeout;

#[impl_cast::device(As: OnMqtt + OnPresence + OnDarkness + OnNotification + OnOff + Timeout)]
pub trait Device: AsGoogleHomeDevice + std::fmt::Debug + Sync + Send {
    fn get_id(&self) -> &str;
}
