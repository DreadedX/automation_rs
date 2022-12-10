pub trait StateOnOff {
    fn set_state(&mut self, state: bool);
    fn get_state(&self) -> bool;
}
