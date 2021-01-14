pub enum RokuKey {
  Power,
  Home,

  Back,
  Ok,

  PadUp,
  PadDown,
  PadLeft,
  PadRight,

  InstantReplay,
  Info,

  VolumeUp,
  VolumeDown,
  VolumeMute
}

impl From<&RokuKey> for &'static str {
  fn from(key: &RokuKey) -> &'static str {
    match key {
      RokuKey::Power         => "power",
      RokuKey::Home          => "home",
      RokuKey::Back          => "back",
      RokuKey::Ok            => "select",
      RokuKey::PadUp         => "up",
      RokuKey::PadDown       => "down",
      RokuKey::PadLeft       => "left",
      RokuKey::PadRight      => "right",
      RokuKey::InstantReplay => "instantreplay",
      RokuKey::Info          => "info",
      RokuKey::VolumeUp      => "volumeup",
      RokuKey::VolumeDown    => "volumedown",
      RokuKey::VolumeMute    => "volumemute"
    }
  }
}

impl From<RokuKey> for &'static str {
  fn from(key: RokuKey) -> &'static str {
    (&key).into()
  }
}