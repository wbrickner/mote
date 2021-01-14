use super::roku::{RokuInput, RokuKey};

pub enum DeviceInput {
  Roku(RokuInput)
}

impl From<RokuKey> for DeviceInput {
  fn from(key: RokuKey) -> DeviceInput {
    DeviceInput::Roku(RokuInput::KeyPress(key))
  }
}