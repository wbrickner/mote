pub mod device_type;
pub mod device_input;
pub mod device_info;
pub mod discovery; pub use discovery::discover;
pub mod roku;

use device_type::*;
use device_input::*;
use device_info::*;
use discovery::*;
use roku::*;

#[derive(Debug, Clone)]
pub struct Device {
  _variant: DeviceType,
  location: std::net::SocketAddr,
  info: DeviceInfo
}

impl Device {
  pub fn ip_string(&self) -> String { self.location.ip().to_string() }

  pub fn device_info(&self) -> &DeviceInfo { &self.info }
  
  pub fn send_input(&self, input: DeviceInput) {
    match input {
      DeviceInput::Roku(i) => {
        let uri = format!("http://{}/{}", self.location, String::from(&i));
        tokio::spawn(async move {
          CLIENT
            .post(uri)
            .send()
            .await
            .expect("dropped input keypress");
        });
      }
    }
  }
}