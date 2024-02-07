use ssdp::{FieldMap, header::{HeaderMut, Man, MX, ST}, message::{SearchRequest, Multicast}};
use futures::future::join_all;
use tokio::{spawn, sync::mpsc::{unbounded_channel, UnboundedReceiver}};
use super::{Device, DeviceInfo, DeviceType, Model, Network, NetworkType, Product, RokuDeviceInfo, System, Uptime};
use static_init::dynamic;

#[dynamic] pub static CLIENT: reqwest::Client = reqwest::Client::new();

pub fn discover() -> UnboundedReceiver<Device> {
  let (tx, rx) = unbounded_channel::<Device>();
  
  spawn(async move {
    // This is not fully correct. In principle devices could change (or worse, swap!) IP addresses during the lifetime
    // of this utility.  This is good enough for now.  Submit a PR if you like.
    let mut devices = vec![];

    let mut request = {
      let mut rq = SearchRequest::new();
      rq.set(Man);
      rq.set(MX(0));
      rq.set(ST::Target(FieldMap::new("roku:ecp").unwrap()));
      rq
    };

    loop {
      // lookup each device independently for lower latency
      let mut fut = vec![];

      for (_, addr) in request.multicast().unwrap() {
        if devices.contains(&addr) { continue; }
        devices.push(addr);

        let tx = tx.clone();
        fut.push(
          spawn(async move {
            let _ = tx.send(device_info(addr).await.unwrap());
          })
        );
      }

      spawn(async move { join_all(fut).await });
    }
  });

  rx
}

/// Gets detailed device info over HTTP.
async fn device_info(location: std::net::SocketAddr) -> anyhow::Result<Device> {
  let response = 
    CLIENT
      .get(format!("http://{}:8060/query/device-info", location.ip()))
      .send()
      .await?
      .text()
      .await?;

  let info: DeviceInfo = {
    let RokuDeviceInfo { 
      name, 
      alternate_name, 
      model_name,
      model_number: number, 
      vendor_name: vendor, 
      serial_number, 

      network_type, 
      network_name, 

      wifi_mac_address, 
      ethernet_mac_address, 
      uptime_seconds 
    } = serde_xml_rs::from_str(&response)?;

    let network_type = NetworkType::from(network_type.as_str());
    let network_name = network_name.unwrap_or_default();

    DeviceInfo {
      name,
      product: Product {
        vendor,
        serial_number,
        model: Model {
          number,
          alternate_name,
          name: model_name,
        }
      },
      network: Network {
        mac_address: match network_type {
          NetworkType::WiFi     => wifi_mac_address,
          NetworkType::Ethernet => ethernet_mac_address,
          _ => wifi_mac_address.or(ethernet_mac_address)
        }.unwrap_or_default(),
        network_type,
        network_name
      },
      system: System {
        uptime: Some(Uptime::new(uptime_seconds))
      }
    }
  };

  // re-use location, just configure port to be correct for Roku devices
  let mut location = location; location.set_port(8060);

  // intermediate struct => general device struct
  Ok(Device {
    _variant: DeviceType::Roku,
    location,
    info
  })
}
