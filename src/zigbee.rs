#[derive(Debug)]
pub struct Zigbee {
    friendly_name: String,
    // manufacturer: String,
    topic: String,
}

impl Zigbee {
    pub fn new(friendly_name: &str, topic: &str) -> Self {
        Self {
            friendly_name: friendly_name.to_owned(),
            // manufacturer: String::from("IKEA"),
            topic: topic.to_owned(),
        }
    }

    pub fn get_friendly_name(&self) -> &str {
        &self.friendly_name
    }

    pub fn get_topic(&self) -> &str {
        &self.topic
    }
}
