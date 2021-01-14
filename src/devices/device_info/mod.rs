mod product;
mod model;
mod network;
mod uptime;
mod system;

pub use product::*;
pub use model::*;
pub use network::*;
pub use uptime::*;
pub use system::*;

#[derive(Debug, Clone)]
pub struct DeviceInfo {
  pub name: String,
  pub product: Product,
  pub network: Network,
  pub system: System
}