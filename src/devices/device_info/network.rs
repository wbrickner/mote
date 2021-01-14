use std::fmt::{Display, Formatter, Result};

/// Describes the link-layer technology that supports the network connection.
/// Presently the only variant is 'WiFi', but for the unfamiliar this could also
/// have a variant called 'Ethernet' and others.
#[derive(Debug, Clone)]
pub enum NetworkType {
  WiFi,
  Unknown
}

impl Display for NetworkType {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    write!(
      f, "{}",
      match self {
        NetworkType::WiFi => "WiFi",
        NetworkType::Unknown => "Unknown"
      }
    )
  }
}

/// Describes the network this device is connected to
#[derive(Debug, Clone)]
pub struct Network {
  pub network_type: NetworkType,
  pub network_name: String,
  pub mac_address: String
}