use hyper::{Client, Request, Method, Body, client::HttpConnector};

mod device_type;
mod device_input;
mod device_info;
mod discovery;
mod roku;

pub use device_type::*;
pub use device_input::*;
pub use device_info::*;
pub use discovery::*;
pub use roku::*;

#[derive(Debug, Clone)]
pub struct Device {
  variant: DeviceType,
  location: std::net::SocketAddr,
  info: DeviceInfo
}

impl Device {
  fn supply_roku_input(&self, roku_input: RokuInput) {
    // ¯\_(ツ)_/¯ this is not my fault. 
    // the Roku engineers decided the API would work over HTTP and you must do a full HTTP request
    // for each each keypress. No streaming, no persistent connection, no authentication, just HTTP.

    // start a brand new Tokio task to see this request through to completion 
    // while the rest of the application forgets it exists

    let uri = format!("http://{}/{}", self.location, String::from(&roku_input));
    // println!("Requesting to uri: {}", uri);

    tokio::spawn(async move {
      // provide custom connector with TCP_NODELAY *enabled* (to prevent delays from packet aggregation,
      // hopefully providing a more fluid exprience)
      let mut custom_connector = HttpConnector::new();
      custom_connector.set_nodelay(true);
      let client = Client::builder().build(custom_connector);

      // ignore HTTP errors
      client.request(
        Request::builder()
          .method(Method::POST)
          .uri(uri)
          .body(Body::empty())
          .expect("Failed to construct request when supplying Roku input")
      )
      .await
      .unwrap_or_else(|e| panic!("Error sending Roku input: {:#?}", e));
    });
  }

  pub fn supply_input(&self, input: DeviceInput) {
    // ignore failed HTTP requests and the like, don't panic the entire interface lol
    match input {
      DeviceInput::Roku(i) => self.supply_roku_input(i)
    }
  }

  pub fn device_info(&self) -> &DeviceInfo { &self.info }

  pub fn ip(&self) -> String {
    self.location.ip().to_string()
  }
}