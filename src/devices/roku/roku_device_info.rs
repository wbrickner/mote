/// Roku-specific representation of device info
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename="device-info")]
pub struct RokuDeviceInfo {
  /// Human-readable device name
  #[serde(rename="friendly-device-name")]
  pub name: String,

  /// Human-readable vendor name
  #[serde(rename="vendor-name")]
  pub vendor_name: String,

  /// Human-readable model brand name
  #[serde(rename="friendly-model-name")]
  pub model_name: String,

  /// Internal model name
  #[serde(rename="model-name")]
  pub alternate_name: String,

  #[serde(rename="model-number")]
  pub model_number: String,

  #[serde(rename="serial-number")]
  pub serial_number: String,

  #[serde(rename="network-type")]
  pub network_type: String,

  #[serde(rename="network-name")]
  pub network_name: Option<String>,

  /// Wireless MAC address (if this device uses WiFi)
  #[serde(rename="wifi-mac")]
  pub wifi_mac_address: Option<String>,

  /// Etherent MAC address (if this device uses Ethernet)
  #[serde(rename="ethernet-mac")]
  pub ethernet_mac_address: Option<String>,

  #[serde(rename="uptime")]
  pub uptime_seconds: u64
}
