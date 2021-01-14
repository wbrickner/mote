use super::RokuKey;

// this is left as an enum because there are other events available to us,
// namely a KeyUp, KeyDown event, but these are not used yet.
pub enum RokuInput {
  KeyPress(RokuKey)
}

impl From<&RokuInput> for String {
  fn from(input: &RokuInput) -> String {
    // NOTE: the /keydown/:k and /keyup/:k routes also exist but are not generated here
    
    let (route, key): (&'static str, &'static str) = match input {
      RokuInput::KeyPress(key) => ("keypress", key.into())
    };
    
    format!("{}/{}", route, key)
  }
}

impl From<RokuInput> for String {
  fn from(input: RokuInput) -> String {
    String::from(&input)
  }
}