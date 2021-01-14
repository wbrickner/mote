use serde::{Deserialize};

/// Roku-specific representation of device info
#[derive(Debug, Deserialize, Clone)]
#[serde(rename="device-info")]
pub struct RokuDeviceInfo {
  #[serde(rename="friendly-device-name")]
  pub name: String,

  #[serde(rename="friendly-model-name")]
  pub model_name_human: String,

  #[serde(rename="model-name")]
  pub model_name: String,

  #[serde(rename="model-number")]
  pub model_number: String,

  #[serde(rename="serial-number")]
  pub serial_number: String,

  #[serde(rename="vendor-name")]
  pub vendor_name: String,

  #[serde(rename="network-type")]
  pub network_type: String,

  #[serde(rename="network-name")]
  pub network_name: String,
  
  #[serde(rename="wifi-mac")]
  pub mac_address: String,

  #[serde(rename="uptime")]
  pub uptime_seconds: u64
}
