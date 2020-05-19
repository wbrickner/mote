extern crate serde;
extern crate serde_xml_rs;
extern crate hyper;

use ssdp::{
  FieldMap,
  header::{HeaderMut, Man, MX, ST},
  message::{SearchRequest, Multicast}
};
use futures::{
  future::{join_all, Abortable, AbortHandle, Aborted},
  channel::mpsc::UnboundedSender
};
use hyper::{
  Client, Request, Method, Body,
  client::{ResponseFuture, HttpConnector}
};

#[derive(Debug, Deserialize, Clone)]
#[serde(rename="device-info")]
pub struct RokuDeviceInfo {
  #[serde(rename="friendly-device-name")]
  human_name: String,

  #[serde(rename="friendly-model-name")]
  human_model_name: String,

  #[serde(rename="model-name")]
  model_name: String,

  #[serde(rename="model-number")]
  model_number: String, 

  #[serde(rename="serial-number")]
  serial_number: String,

  #[serde(rename="vendor-name")]
  vendor_name: String,

  #[serde(rename="network-type")]
  network_type: String,

  #[serde(rename="network-name")]
  network_name: String,
  
  #[serde(rename="wifi-mac")]
  mac_address: String,

  #[serde(rename="uptime")]
  uptime_seconds: u64
}

#[derive(Debug, Clone)]
pub enum DeviceVariant {
  Roku(RokuDeviceInfo),
  // add other device types...
  // GoogleCast(GoogleCastDeviceInfo)
}

pub enum DeviceInput {
  Roku(RokuInput)
}

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

impl RokuKey {
  fn to_str(&self) -> &'static str {
    match self {
      RokuKey::Power => "power",
      RokuKey::Home => "home",
      RokuKey::Back => "back",
      RokuKey::Ok => "select",
      RokuKey::PadUp => "up",
      RokuKey::PadDown => "down",
      RokuKey::PadLeft => "left",
      RokuKey::PadRight => "right",
      RokuKey::InstantReplay => "instantreplay",
      RokuKey::Info => "info",
      RokuKey::VolumeUp => "volumeup",
      RokuKey::VolumeDown => "volumedown",
      RokuKey::VolumeMute => "volumemute"
    }
  }
}

pub enum RokuInput {
  KeyDown(RokuKey),
  KeyUp(RokuKey),
  KeyPress(RokuKey)
}

impl RokuInput {
  // converts entire input to a url path ready for use
  fn to_str(&self) -> String {
    match self {
      RokuInput::KeyDown(key) => format!("keydown/{}", key.to_str()),
      RokuInput::KeyUp(key) => format!("keyup/{}", key.to_str()),
      RokuInput::KeyPress(key) => format!("keypress/{}", key.to_str())
    }
  }
}

#[derive(Debug, Clone)]
pub struct Device {
  location: std::net::SocketAddr,
  variant: DeviceVariant,
  http_client: Client<HttpConnector>
}

impl Device {
  fn new(location: std::net::SocketAddr, variant: DeviceVariant) -> Self {
    // provide custom connector with TCP_NODELAY *enabled* (to prevent delays from packet aggregation)
    let mut custom_connector = HttpConnector::new();
    custom_connector.set_nodelay(true);

    Device { 
      location, 
      variant,
      http_client: Client::builder()
                      .build(custom_connector)
    }
  }

  async fn supply_roku_input(&self, roku_input: RokuInput) -> Result<hyper::Response<hyper::Body>, hyper::Error> {
    // automatically return Err()
    self.http_client.request(
    //  Client::new().request(
      Request::builder()
        .method(Method::POST)
        .uri(format!(
          "http://{}/{}",
          self.location,
          roku_input.to_str()
        ))
        .body(Body::empty())
        .expect("Failed to construct request")
    ).await
  }

  pub async fn supply_input(&self, input: DeviceInput) -> Result<(), ()> {
    match input {
      DeviceInput::Roku(ri) => match self.supply_roku_input(ri).await {
        Ok(_) => Ok(()),
        Err(_) => Err(())
      },
      // add additional device types here
    }
  }

  pub fn get_human_name<'a>(&'a self) -> &'a String {
    match &self.variant {
      DeviceVariant::Roku(roku_device_info) => &roku_device_info.human_name
    }
  }

  pub fn get_human_model_name<'a>(&'a self) -> &'a String {
    match &self.variant {
      DeviceVariant::Roku(roku_device_info) => &roku_device_info.human_model_name
    }
  }

  pub fn get_ip(&self) -> String {
    format!("{}", self.location.ip())
  }
}

async fn lookup_roku_device_info(mut location: std::net::SocketAddr) -> Device {
  let request = Client::new().request(
      Request::builder()
        .method(Method::GET)
        .uri(
          format!("http://{}:8060/query/device-info", location.ip())
        )
        .body(Body::empty())
        .expect("Failed to construct request")
    ).await.expect("Failed to GET device info");

  // response => byte slice => str
  let response_bytes = hyper::body::to_bytes(request.into_body()).await.expect("Failed to interpret device info response");
  let response_str = std::str::from_utf8(&response_bytes[..]).expect("Failed to interpret device info response as string");

  // xml str => intermediate struct
  let device_info: RokuDeviceInfo = serde_xml_rs::from_str(response_str).expect("Failed to parse device info from response");

  // re-use location, just configure port to be correct for Roku devices
  location.set_port(8060);

  // intermediate struct => agnostic public struct
  Device::new(
    location,
    DeviceVariant::Roku(device_info)
  )
}

pub fn begin_device_discovery<'a>(roku_discovery_tx: UnboundedSender<Device>) {
  tokio::spawn(async move {
    let mut discovered_devices = Vec::<std::net::SocketAddr>::new();

    loop {
      let mut request = SearchRequest::new();

      request.set(Man);
      request.set(MX(0));
      request.set(ST::Target(FieldMap::new("roku:ecp").unwrap()));

      // TODO: report device locations as they're discovered, do each lookup independently,
      //       this way the user can see a device and a "Loading..." placeholder for device info
      //       and reduce the perceived latency
      let mut lookup_futures = Vec::new();

      for (_, location) in request.multicast().unwrap() {
        if discovered_devices.contains(&location) { continue; }
        discovered_devices.push(location);

        // begin async lookup of device info
        lookup_futures.push(lookup_roku_device_info(location));
      }

      let lookup_results = join_all(lookup_futures).await;

      for result in lookup_results.iter() {
        roku_discovery_tx
          .unbounded_send(result.clone())
          .expect("Failed to send host along discovery pipe")
      }
    }
  });
}