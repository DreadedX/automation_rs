use google_home::{errors::ErrorCode, traits};

use super::Device;

pub struct TestOutlet {
    on: bool
}

impl TestOutlet {
    pub fn new() -> Self {
        Self { on: false }
    }
}

impl Device for TestOutlet {
    fn get_id(&self) -> String {
        "test_device".into()
    }
}

impl traits::OnOff for TestOutlet {
    fn is_on(&self) -> Result<bool, ErrorCode> {
        Ok(self.on)
    }

    fn set_on(&mut self, on: bool) -> Result<(), ErrorCode> {
        println!("Setting on: {on}");
        self.on = on;
        Ok(())
    }
}
