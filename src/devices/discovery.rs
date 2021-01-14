use std::{net::SocketAddr, str::from_utf8, time::Duration};
use futures::channel::mpsc::{unbounded, UnboundedSender, UnboundedReceiver};
use hyper::{Body, Client, Method, Request, body::to_bytes};
use ssdp::{FieldMap, header::{HeaderMut, Man, MX, ST}, message::{SearchRequest, Multicast}};
use futures::{future::join_all};
use tokio::{spawn, time::delay_for};

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
    tokio::spawn(async move {
      // This is not fully correct. In principle devices could change (or worse, swap!) IP addresses during the lifetime
      // of this utility.  This is good enough for now.  Submit a PR if you like.
      let mut discovered_devices = Vec::<SocketAddr>::new();
      let half_second = Duration::from_millis(500);
      
      // This is entirely Roku-specific nonsense
      loop {
        let mut request = SearchRequest::new();
        request.set(Man);
        request.set(MX(0));
        request.set(ST::Target(FieldMap::new("roku:ecp").unwrap()));

        // consider reporting device locations as they're discovered, do each lookup independently,
        // this way the user can see a device and a "Loading..." placeholder for device info
        // and reduce the perceived latency
        let mut lookup_futures = vec![];
        for (_, location) in request.multicast().unwrap() {
          if discovered_devices.contains(&location) { continue; }
          discovered_devices.push(location);

          lookup_futures.push(Discoverer::lookup_roku_device_info(location));
        }
          

        // fancy pants spawns a new task so old lookups dont block new discoveries
        let tx = discovery_tx.clone();
        spawn(async move {
          join_all(lookup_futures)
            .await
            .iter()
            .for_each(|result|
              tx
                .unbounded_send(result.clone())
                .expect("Failed to forward device lookup result along discovery pipe")
            );
        });

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

  async fn lookup_roku_device_info(mut location: std::net::SocketAddr) -> Device {
    let request = Client::new().request(
        Request::builder()
          .method(Method::GET)
          .uri(
            format!("http://{}:8060/query/device-info", location.ip())
          )
          .body(Body::empty())
          .expect("Failed to construct request when investigating Roku device info")
      )
      .await
      .expect("Failed to get Roku device info");

    // Unfortunately there is no way to parse directly from bytes so
    // we must use a conversion to get our hands on a (verified) utf8 str
    let response_bytes = to_bytes(request.into_body()).await.expect("Failed to interpret device info response as bytes");
    let response_str = from_utf8(&response_bytes).expect("Failed to interpret device info response as str of utf8 chars");

    let info: DeviceInfo = {
      let roku_info: RokuDeviceInfo = serde_xml_rs::from_str(response_str).expect("Failed to parse device info from response");
      
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
          network_type: match roku_info.network_type.as_str() {
            "wifi" => NetworkType::WiFi,
            _ => NetworkType::Unknown
          },
          network_name: roku_info.network_name,
          mac_address: roku_info.mac_address
        },
        system: System {
          uptime: Some(Uptime::new(roku_info.uptime_seconds))
        }
      }
    };

    // re-use location, just configure port to be correct for Roku devices
    location.set_port(8060);

    // intermediate struct => general device struct
    Device {
      variant: DeviceType::Roku,
      location,
      info
    }
  }
}