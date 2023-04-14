use async_trait::async_trait;
use impl_cast::device_trait;
use rumqttc::Publish;

use crate::ntfy::Notification;

#[async_trait]
#[device_trait]
pub trait OnMqtt {
    fn topics(&self) -> Vec<&str>;
    async fn on_mqtt(&mut self, message: Publish);
}

#[async_trait]
#[device_trait]
pub trait OnPresence {
    async fn on_presence(&mut self, presence: bool);
}

#[async_trait]
#[device_trait]
pub trait OnDarkness {
    async fn on_darkness(&mut self, dark: bool);
}

#[async_trait]
#[device_trait]
pub trait OnNotification {
    async fn on_notification(&mut self, notification: Notification);
}
