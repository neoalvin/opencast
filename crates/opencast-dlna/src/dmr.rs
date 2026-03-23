use crate::xml_templates;
use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use opencast_core::{PositionInfo, TransportState, VolumeInfo};
use opencast_discovery::ssdp::build_device_description;
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{debug, error, info, warn};

/// Callback for handling media commands from a DLNA controller.
pub trait RendererCallback: Send + Sync + 'static {
    fn on_set_uri(&self, url: String, metadata: String);
    fn on_play(&self);
    fn on_pause(&self);
    fn on_stop(&self);
    fn on_seek(&self, position_secs: f64);
    fn on_set_volume(&self, volume: u32);
    fn on_set_mute(&self, muted: bool);
    fn get_position_info(&self) -> PositionInfo;
    fn get_transport_state(&self) -> TransportState;
    fn get_volume_info(&self) -> VolumeInfo;
}

/// DLNA Digital Media Renderer (DMR).
/// Runs an HTTP server that accepts SOAP control commands from a DLNA controller.
/// This is the "receiver" side — runs on the TV/display device.
pub struct DlnaRenderer {
    friendly_name: String,
    udn: String,
    port: u16,
    callback: Arc<dyn RendererCallback>,
}

impl DlnaRenderer {
    pub fn new(
        friendly_name: impl Into<String>,
        port: u16,
        callback: Arc<dyn RendererCallback>,
    ) -> Self {
        Self {
            friendly_name: friendly_name.into(),
            udn: uuid::Uuid::new_v4().to_string(),
            port,
            callback,
        }
    }

    pub fn udn(&self) -> &str {
        &self.udn
    }

    /// Start the DLNA renderer HTTP server and SSDP advertisement.
    pub async fn start(self) -> anyhow::Result<()> {
        let addr = SocketAddr::from(([0, 0, 0, 0], self.port));
        let listener = TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;

        let local_ip = get_local_ip().unwrap_or(Ipv4Addr::LOCALHOST);
        let base_url = format!("http://{}:{}", local_ip, local_addr.port());

        info!(
            "DLNA Renderer '{}' starting on {}",
            self.friendly_name, base_url
        );

        let state = Arc::new(RendererState {
            friendly_name: self.friendly_name.clone(),
            udn: self.udn.clone(),
            base_url: base_url.clone(),
            callback: self.callback.clone(),
        });

        // Start SSDP advertisement in background
        let ssdp_state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = advertise_ssdp(&ssdp_state).await {
                error!("SSDP advertisement error: {e}");
            }
        });

        // Accept HTTP connections
        info!("DLNA Renderer ready. Waiting for connections...");
        loop {
            let (stream, peer) = listener.accept().await?;
            debug!("Connection from {peer}");

            let state = state.clone();
            tokio::spawn(async move {
                let io = TokioIo::new(stream);
                let service = service_fn(move |req| {
                    let state = state.clone();
                    async move { handle_request(req, state).await }
                });

                if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                    debug!("HTTP connection error: {e}");
                }
            });
        }
    }
}

struct RendererState {
    friendly_name: String,
    udn: String,
    base_url: String,
    callback: Arc<dyn RendererCallback>,
}

async fn handle_request(
    req: Request<Incoming>,
    state: Arc<RendererState>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    debug!("{method} {path}");

    let response = match path.as_str() {
        "/description.xml" => {
            let xml = build_device_description(
                &state.friendly_name,
                &state.udn,
                &state.base_url,
            );
            ok_xml(xml)
        }
        "/AVTransport/scpd.xml" => ok_xml(xml_templates::av_transport_scpd().to_string()),
        "/RenderingControl/scpd.xml" => {
            ok_xml(xml_templates::rendering_control_scpd().to_string())
        }
        "/ConnectionManager/scpd.xml" => {
            ok_xml(xml_templates::connection_manager_scpd().to_string())
        }
        "/AVTransport/control" => {
            let body = read_body(req).await;
            handle_av_transport(&body, &state)
        }
        "/RenderingControl/control" => {
            let body = read_body(req).await;
            handle_rendering_control(&body, &state)
        }
        "/ConnectionManager/control" => {
            let body = read_body(req).await;
            handle_connection_manager(&body)
        }
        "/AVTransport/event" | "/RenderingControl/event" | "/ConnectionManager/event" => {
            // GENA event subscription — return 200 for now
            Response::builder()
                .status(StatusCode::OK)
                .header("SID", format!("uuid:{}", uuid::Uuid::new_v4()))
                .header("TIMEOUT", "Second-300")
                .body(Full::new(Bytes::new()))
                .unwrap()
        }
        _ => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Full::new(Bytes::from("Not Found")))
            .unwrap(),
    };

    Ok(response)
}

fn handle_av_transport(body: &str, state: &RendererState) -> Response<Full<Bytes>> {
    let action = extract_soap_action(body);
    debug!("AVTransport action: {action}");

    match action.as_str() {
        "SetAVTransportURI" => {
            let uri = extract_tag_value(body, "CurrentURI").unwrap_or_default();
            let metadata = extract_tag_value(body, "CurrentURIMetaData").unwrap_or_default();
            info!("SetAVTransportURI: {uri}");
            state.callback.on_set_uri(uri, metadata);
            soap_response("SetAVTransportURI", "")
        }
        "Play" => {
            state.callback.on_play();
            soap_response("Play", "")
        }
        "Pause" => {
            state.callback.on_pause();
            soap_response("Pause", "")
        }
        "Stop" => {
            state.callback.on_stop();
            soap_response("Stop", "")
        }
        "Seek" => {
            let target = extract_tag_value(body, "Target").unwrap_or_default();
            let secs = xml_templates::parse_duration(&target);
            state.callback.on_seek(secs);
            soap_response("Seek", "")
        }
        "GetPositionInfo" => {
            let info = state.callback.get_position_info();
            let response_body = format!(
                "<Track>1</Track>\
                 <TrackDuration>{}</TrackDuration>\
                 <TrackMetaData></TrackMetaData>\
                 <TrackURI>{}</TrackURI>\
                 <RelTime>{}</RelTime>\
                 <AbsTime>{}</AbsTime>\
                 <RelCount>0</RelCount>\
                 <AbsCount>0</AbsCount>",
                xml_templates::format_duration(info.duration),
                info.track_uri.as_deref().unwrap_or(""),
                xml_templates::format_duration(info.position),
                xml_templates::format_duration(info.position),
            );
            soap_response("GetPositionInfo", &response_body)
        }
        "GetTransportInfo" => {
            let ts = state.callback.get_transport_state();
            let response_body = format!(
                "<CurrentTransportState>{}</CurrentTransportState>\
                 <CurrentTransportStatus>OK</CurrentTransportStatus>\
                 <CurrentSpeed>1</CurrentSpeed>",
                ts,
            );
            soap_response("GetTransportInfo", &response_body)
        }
        _ => {
            warn!("Unhandled AVTransport action: {action}");
            soap_response(&action, "")
        }
    }
}

fn handle_rendering_control(body: &str, state: &RendererState) -> Response<Full<Bytes>> {
    let action = extract_soap_action(body);
    debug!("RenderingControl action: {action}");

    match action.as_str() {
        "SetVolume" => {
            let vol: u32 = extract_tag_value(body, "DesiredVolume")
                .and_then(|v| v.parse().ok())
                .unwrap_or(50);
            state.callback.on_set_volume(vol);
            soap_response("SetVolume", "")
        }
        "GetVolume" => {
            let info = state.callback.get_volume_info();
            let vol = (info.level * 100.0) as u32;
            soap_response("GetVolume", &format!("<CurrentVolume>{vol}</CurrentVolume>"))
        }
        "SetMute" => {
            let muted = extract_tag_value(body, "DesiredMute")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            state.callback.on_set_mute(muted);
            soap_response("SetMute", "")
        }
        "GetMute" => {
            let info = state.callback.get_volume_info();
            let muted = if info.muted { "1" } else { "0" };
            soap_response("GetMute", &format!("<CurrentMute>{muted}</CurrentMute>"))
        }
        _ => {
            warn!("Unhandled RenderingControl action: {action}");
            soap_response(&action, "")
        }
    }
}

fn handle_connection_manager(body: &str) -> Response<Full<Bytes>> {
    let action = extract_soap_action(body);
    debug!("ConnectionManager action: {action}");

    match action.as_str() {
        "GetProtocolInfo" => {
            let sink = "http-get:*:video/mp4:*,\
                         http-get:*:video/x-matroska:*,\
                         http-get:*:video/webm:*,\
                         http-get:*:video/avi:*,\
                         http-get:*:audio/mpeg:*,\
                         http-get:*:audio/mp4:*,\
                         http-get:*:audio/flac:*,\
                         http-get:*:audio/wav:*,\
                         http-get:*:image/jpeg:*,\
                         http-get:*:image/png:*";
            soap_response(
                "GetProtocolInfo",
                &format!("<Source></Source><Sink>{sink}</Sink>"),
            )
        }
        _ => soap_response(&action, ""),
    }
}

fn soap_response(action: &str, body: &str) -> Response<Full<Bytes>> {
    let xml = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/"
            s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
  <s:Body>
    <u:{action}Response xmlns:u="urn:schemas-upnp-org:service:AVTransport:1">
      {body}
    </u:{action}Response>
  </s:Body>
</s:Envelope>"#,
        action = action,
        body = body,
    );

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/xml; charset=\"utf-8\"")
        .body(Full::new(Bytes::from(xml)))
        .unwrap()
}

fn ok_xml(xml: String) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/xml; charset=\"utf-8\"")
        .body(Full::new(Bytes::from(xml)))
        .unwrap()
}

async fn read_body(req: Request<Incoming>) -> String {
    use http_body_util::BodyExt;
    let body = req.collect().await.map(|c| c.to_bytes()).unwrap_or_default();
    String::from_utf8_lossy(&body).to_string()
}

fn extract_soap_action(body: &str) -> String {
    // Look for <u:ActionName ...> pattern
    if let Some(pos) = body.find("<u:") {
        let rest = &body[pos + 3..];
        if let Some(end) = rest.find(|c: char| c == ' ' || c == '>' || c == '/') {
            return rest[..end].to_string();
        }
    }
    "Unknown".to_string()
}

fn extract_tag_value(body: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    if let Some(start) = body.find(&open) {
        let value_start = start + open.len();
        if let Some(end) = body[value_start..].find(&close) {
            return Some(body[value_start..value_start + end].to_string());
        }
    }
    // Also try with namespace prefix
    let open_ns = format!(":{tag}>");
    if let Some(start) = body.find(&open_ns) {
        let value_start = start + open_ns.len();
        let close_ns = format!(":{tag}>");
        // Find the closing tag with any namespace prefix
        if let Some(end) = body[value_start..].find(&format!("</{tag}>")) {
            return Some(body[value_start..value_start + end].to_string());
        }
        if let Some(end_pos) = body[value_start..].find("</") {
            let remaining = &body[value_start + end_pos..];
            if remaining.contains(&close_ns) {
                return Some(body[value_start..value_start + end_pos].to_string());
            }
        }
    }
    None
}

/// SSDP NOTIFY advertisement - announce this renderer on the network.
async fn advertise_ssdp(state: &RendererState) -> anyhow::Result<()> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    socket.set_reuse_address(true)?;
    #[cfg(unix)]
    socket.set_reuse_port(true)?;
    socket.set_multicast_ttl_v4(4)?;
    socket.bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0).into())?;

    let std_socket: std::net::UdpSocket = socket.into();
    std_socket.set_nonblocking(true)?;
    let udp = tokio::net::UdpSocket::from_std(std_socket)?;

    let dest: SocketAddr = SocketAddrV4::new(Ipv4Addr::new(239, 255, 255, 250), 1900).into();
    let description_url = format!("{}/description.xml", state.base_url);

    // Also listen for M-SEARCH requests and respond
    let search_socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
    search_socket.set_reuse_address(true)?;
    #[cfg(unix)]
    search_socket.set_reuse_port(true)?;
    search_socket
        .join_multicast_v4(&Ipv4Addr::new(239, 255, 255, 250), &Ipv4Addr::UNSPECIFIED)?;
    search_socket.bind(&SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 1900).into())?;

    let std_search: std::net::UdpSocket = search_socket.into();
    std_search.set_nonblocking(true)?;
    let search_udp = tokio::net::UdpSocket::from_std(std_search)?;

    let description_url_clone = description_url.clone();
    let udn = state.udn.clone();


    // Spawn M-SEARCH responder
    tokio::spawn(async move {
        let mut buf = [0u8; 4096];
        loop {
            match search_udp.recv_from(&mut buf).await {
                Ok((len, peer)) => {
                    let msg = String::from_utf8_lossy(&buf[..len]);
                    if msg.contains("M-SEARCH") &&
                       (msg.contains("MediaRenderer") || msg.contains("ssdp:all") || msg.contains("upnp:rootdevice"))
                    {
                        let response = format!(
                            "HTTP/1.1 200 OK\r\n\
                             CACHE-CONTROL: max-age=1800\r\n\
                             LOCATION: {}\r\n\
                             SERVER: OpenCast/0.1 UPnP/1.0\r\n\
                             ST: urn:schemas-upnp-org:device:MediaRenderer:1\r\n\
                             USN: uuid:{}::urn:schemas-upnp-org:device:MediaRenderer:1\r\n\
                             \r\n",
                            description_url_clone, udn
                        );
                        let _ = search_udp.send_to(response.as_bytes(), peer).await;
                        debug!("Responded to M-SEARCH from {peer}");
                    }
                }
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        }
    });

    // Periodic SSDP NOTIFY alive
    loop {
        let notify = format!(
            "NOTIFY * HTTP/1.1\r\n\
             HOST: 239.255.255.250:1900\r\n\
             CACHE-CONTROL: max-age=1800\r\n\
             LOCATION: {}\r\n\
             NT: urn:schemas-upnp-org:device:MediaRenderer:1\r\n\
             NTS: ssdp:alive\r\n\
             SERVER: OpenCast/0.1 UPnP/1.0\r\n\
             USN: uuid:{}::urn:schemas-upnp-org:device:MediaRenderer:1\r\n\
             \r\n",
            description_url, state.udn
        );

        let _ = udp.send_to(notify.as_bytes(), dest).await;
        debug!("Sent SSDP NOTIFY alive");

        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

/// Get the primary local IPv4 address.
fn get_local_ip() -> Option<Ipv4Addr> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    match socket.local_addr().ok()? {
        SocketAddr::V4(addr) => Some(*addr.ip()),
        _ => None,
    }
}
