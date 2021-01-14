use std::fmt::{Display, Formatter, Result};

/// Describes the link-layer technology that supports the network connection.
#[derive(Debug, Clone)]
pub enum NetworkType {
  WiFi,
  Ethernet,
  Unknown
}

impl Display for NetworkType {
  fn fmt(&self, f: &mut Formatter<'_>) -> Result {
    write!(
      f, "{}",
      match self {
        NetworkType::WiFi => "WiFi",
        NetworkType::Ethernet => "Ethernet",
        NetworkType::Unknown => "Unknown"
      }
    )
  }
}

impl From<&str> for NetworkType {
  fn from(literal: &str) -> Self {
    match literal {
      "wifi" => Self::WiFi,
      "ethernet" => Self::Ethernet,
      _ => Self::Unknown
    }
  }
}

/// Describes the network this device is connected to
#[derive(Debug, Clone)]
pub struct Network {
  pub network_type: NetworkType,
  pub network_name: String,
  pub mac_address: String
}