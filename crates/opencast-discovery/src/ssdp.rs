use opencast_core::{Device, DeviceType};
use socket2::{Domain, Protocol, Socket, Type};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

const SSDP_MULTICAST_ADDR: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
const SSDP_PORT: u16 = 1900;
const MEDIA_RENDERER_URN: &str = "urn:schemas-upnp-org:device:MediaRenderer:1";

/// SSDP-based device discovery for UPnP/DLNA.
pub struct SsdpDiscovery {
    devices: Arc<RwLock<HashMap<String, Device>>>,
}

impl Default for SsdpDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

impl SsdpDiscovery {
    pub fn new() -> Self {
        Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Search for DLNA media renderers on the local network.
    /// Returns discovered devices after waiting for `timeout` duration.
    pub async fn search_renderers(&self, timeout: Duration) -> anyhow::Result<Vec<Device>> {
        self.search(MEDIA_RENDERER_URN, timeout).await
    }

    /// Search for UPnP devices matching the given search target.
    pub async fn search(&self, search_target: &str, timeout: Duration) -> anyhow::Result<Vec<Device>> {
        let socket = self.create_socket()?;
        let search_msg = format!(
            "M-SEARCH * HTTP/1.1\r\n\
             HOST: 239.255.255.250:1900\r\n\
             MAN: \"ssdp:discover\"\r\n\
             MX: {}\r\n\
             ST: {}\r\n\
             \r\n",
            timeout.as_secs().max(1),
            search_target
        );

        let dest = SocketAddrV4::new(SSDP_MULTICAST_ADDR, SSDP_PORT);
        let std_socket: std::net::UdpSocket = socket.into();
        std_socket.set_nonblocking(true)?;
        let udp = tokio::net::UdpSocket::from_std(std_socket)?;

        // Send M-SEARCH
        udp.send_to(search_msg.as_bytes(), dest).await?;
        info!("Sent SSDP M-SEARCH for {search_target}");

        // Collect responses
        let devices = self.devices.clone();
        let mut buf = [0u8; 4096];

        let collect = async {
            loop {
                match udp.recv_from(&mut buf).await {
                    Ok((len, addr)) => {
                        let response = String::from_utf8_lossy(&buf[..len]);
                        debug!("SSDP response from {addr}:\n{response}");

                        if let Some(location) = Self::parse_location(&response) {
                            match Self::fetch_device_description(&location).await {
                                Ok(device) => {
                                    info!("Discovered: {device}");
                                    devices.write().await.insert(device.id.clone(), device);
                                }
                                Err(e) => {
                                    warn!("Failed to fetch device description from {location}: {e}");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        debug!("SSDP recv error (may be timeout): {e}");
                        break;
                    }
                }
            }
        };

        tokio::select! {
            _ = collect => {},
            _ = tokio::time::sleep(timeout) => {
                debug!("SSDP search timeout reached");
            }
        }

        let devices = self.devices.read().await;
        Ok(devices.values().cloned().collect())
    }

    /// Get currently known devices.
    pub async fn devices(&self) -> Vec<Device> {
        self.devices.read().await.values().cloned().collect()
    }

    fn create_socket(&self) -> anyhow::Result<Socket> {
        let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
        socket.set_reuse_address(true)?;
        #[cfg(unix)]
        socket.set_reuse_port(true)?;
        socket.set_multicast_ttl_v4(4)?;
        socket.join_multicast_v4(&SSDP_MULTICAST_ADDR, &Ipv4Addr::UNSPECIFIED)?;
        socket.bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0).into())?;
        Ok(socket)
    }

    fn parse_location(response: &str) -> Option<String> {
        for line in response.lines() {
            let lower = line.to_lowercase();
            if lower.starts_with("location:") {
                return Some(line[9..].trim().to_string());
            }
        }
        None
    }

    /// Fetch and parse a UPnP device XML description.
    async fn fetch_device_description(location: &str) -> anyhow::Result<Device> {
        let response = reqwest::get(location).await?.text().await?;
        Self::parse_device_xml(&response, location)
    }

    fn parse_device_xml(xml: &str, location: &str) -> anyhow::Result<Device> {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let mut reader = Reader::from_str(xml);
        let mut buf = Vec::new();
        let mut friendly_name = String::new();
        let mut udn = String::new();
        let mut device_type_str = String::new();
        let mut manufacturer = None;
        let mut model_name = None;
        let mut current_tag = String::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    current_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                }
                Ok(Event::Text(ref e)) => {
                    let text = e.unescape().unwrap_or_default().to_string();
                    match current_tag.as_str() {
                        "friendlyName" => friendly_name = text,
                        "UDN" => udn = text,
                        "deviceType" => device_type_str = text,
                        "manufacturer" => manufacturer = Some(text),
                        "modelName" => model_name = Some(text),
                        _ => {}
                    }
                }
                Ok(Event::End(_)) => {
                    current_tag.clear();
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    warn!("XML parse error: {e}");
                    break;
                }
                _ => {}
            }
            buf.clear();
        }

        if udn.is_empty() {
            udn = uuid::Uuid::new_v4().to_string();
        }

        let device_type = if device_type_str.contains("MediaRenderer") {
            DeviceType::DlnaRenderer
        } else if device_type_str.contains("MediaServer") {
            DeviceType::DlnaServer
        } else {
            DeviceType::DlnaRenderer
        };

        let location_url = url::Url::parse(location)?;

        Ok(Device {
            id: udn,
            name: if friendly_name.is_empty() {
                "Unknown Device".to_string()
            } else {
                friendly_name
            },
            device_type,
            location: location_url,
            manufacturer,
            model_name,
        })
    }
}

/// Build a UPnP device description XML for advertising as a media renderer.
pub fn build_device_description(
    friendly_name: &str,
    udn: &str,
    base_url: &str,
) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<root xmlns="urn:schemas-upnp-org:device-1-0">
  <specVersion>
    <major>1</major>
    <minor>0</minor>
  </specVersion>
  <device>
    <deviceType>urn:schemas-upnp-org:device:MediaRenderer:1</deviceType>
    <friendlyName>{friendly_name}</friendlyName>
    <manufacturer>OpenCast</manufacturer>
    <modelName>OpenCast Renderer</modelName>
    <modelDescription>OpenCast DLNA Media Renderer</modelDescription>
    <UDN>uuid:{udn}</UDN>
    <serviceList>
      <service>
        <serviceType>urn:schemas-upnp-org:service:AVTransport:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:AVTransport</serviceId>
        <controlURL>/AVTransport/control</controlURL>
        <eventSubURL>/AVTransport/event</eventSubURL>
        <SCPDURL>/AVTransport/scpd.xml</SCPDURL>
      </service>
      <service>
        <serviceType>urn:schemas-upnp-org:service:RenderingControl:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:RenderingControl</serviceId>
        <controlURL>/RenderingControl/control</controlURL>
        <eventSubURL>/RenderingControl/event</eventSubURL>
        <SCPDURL>/RenderingControl/scpd.xml</SCPDURL>
      </service>
      <service>
        <serviceType>urn:schemas-upnp-org:service:ConnectionManager:1</serviceType>
        <serviceId>urn:upnp-org:serviceId:ConnectionManager</serviceId>
        <controlURL>/ConnectionManager/control</controlURL>
        <eventSubURL>/ConnectionManager/event</eventSubURL>
        <SCPDURL>/ConnectionManager/scpd.xml</SCPDURL>
      </service>
    </serviceList>
  </device>
  <URLBase>{base_url}</URLBase>
</root>"#,
        friendly_name = friendly_name,
        udn = udn,
        base_url = base_url,
    )
}
