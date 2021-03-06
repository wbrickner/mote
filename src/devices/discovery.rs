use std::{net::SocketAddr, str::from_utf8};
use futures::channel::mpsc::{unbounded, UnboundedSender, UnboundedReceiver};
use hyper::{Body, Client, Method, Request, body::to_bytes};
use ssdp::{FieldMap, header::{HeaderMut, Man, MX, ST}, message::{SearchRequest, Multicast}};
use futures::{future::join_all};
use tokio::spawn;

use super::{Device, DeviceInfo, DeviceType, Model, Network, NetworkType, Product, RokuDeviceInfo, System, Uptime};

#[derive(Debug, Clone)]
pub struct Discoverer { }

impl Discoverer {
  pub fn begin() -> UnboundedReceiver<Device> {
    let (discovery_tx, discovery_rx) = unbounded::<Device>();
    
    Discoverer::spawn_roku_discovery(discovery_tx);
    discovery_rx
  }

  fn spawn_roku_discovery(discovery_tx: UnboundedSender<Device>) {
    spawn(async move {
      // This is not fully correct. In principle devices could change (or worse, swap!) IP addresses during the lifetime
      // of this utility.  This is good enough for now.  Submit a PR if you like.
      let mut discovered_devices = Vec::<SocketAddr>::new();
      
      // This is entirely Roku-specific nonsense
      loop {
        let mut request = SearchRequest::new();
        request.set(Man);
        request.set(MX(0));
        request.set(ST::Target(FieldMap::new("roku:ecp").unwrap()));

        // lookup each device independently for lower latency
        let mut lookup_futures = vec![];
        for (_, location) in request.multicast().unwrap() {
          if discovered_devices.contains(&location) { continue; }
          discovered_devices.push(location);

          // fancy pants spawns a new task for each device so they can complete independently
          let tx = discovery_tx.clone();
          let lookup_future = spawn(async move {
            if let Some(device) = Discoverer::lookup_roku_device_info(location).await {
              tx.unbounded_send(device)
                .expect("Failed to forward device lookup result along discovery channel");
            }
          });

          lookup_futures.push(lookup_future);
        }

        // spawn a new async task to drive the lookup futures, allowing discovery task to get
        // right back to work.
        spawn(async move { join_all(lookup_futures).await });

        // I want very prompt discovery. If I ever have to wait for any stupid piece of technology
        // to get off its lazy $60 to $1000 ass and respond to a multicast DNS request ever again I will fucking lose it,
        // so set this to 500ms (idle sleep)

        // NOTE: the above was written in rage, brought on by the traumatic memory of
        //       literally any experience with any auto-discovery implementation.
        //       It may be more reasonable to do an exponential backoff in the sleep period. TODO.
        // delay_for(half_second).await;
      }
    });
  }

  /// Gets detailed device info over HTTP. If response is unparseable or request fails,
  /// yields None variant.
  async fn lookup_roku_device_info(mut location: std::net::SocketAddr) -> Option<Device> {
    let request = Client::new().request(
        Request::builder()
          .method(Method::GET)
          .uri(
            format!("http://{}:8060/query/device-info", location.ip())
          )
          .body(Body::empty())
          .expect("Failed to construct request when investigating Roku device info")
      )
    .await;

    let request = match request {
      Err(_) => return None,
      Ok(r) => r,
    };

    // Unfortunately there is no way to parse directly from bytes so
    // we must use a conversion to get our hands on a (verified) utf8 str
    let response_bytes = to_bytes(request.into_body()).await.expect("Failed to interpret device info response as bytes");
    let response_str = from_utf8(&response_bytes).expect("Failed to interpret device info response as str of utf8 chars");

    let info: DeviceInfo = {
      let roku_info: RokuDeviceInfo = serde_xml_rs::from_str(response_str).expect("Failed to parse device info from response");
      let network_type: NetworkType = roku_info.network_type.as_str().into();

      DeviceInfo {
        name: roku_info.name,
        product: Product {
          vendor: roku_info.vendor_name,
          model: Model {
            name: roku_info.model_name_human,
            alternate_name: roku_info.model_name,
            number: roku_info.model_number
          },
          serial_number: roku_info.serial_number
        },
        network: Network {
          network_type: network_type.clone(),
          network_name: roku_info.network_name.unwrap_or("".into()),
          mac_address: match network_type {
            NetworkType::WiFi => roku_info.wifi_mac_address.unwrap_or("".into()),
            NetworkType::Ethernet => roku_info.ethernet_mac_address.unwrap_or("".into()),
            _ => "".into()
          }
        },
        system: System {
          uptime: Some(Uptime::new(roku_info.uptime_seconds))
        }
      }
    };

    // re-use location, just configure port to be correct for Roku devices
    location.set_port(8060);

    // intermediate struct => general device struct
    Some(
      Device {
        variant: DeviceType::Roku,
        location,
        info
      }
    )
  }
}