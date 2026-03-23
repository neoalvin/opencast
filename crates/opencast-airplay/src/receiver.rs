use crate::plist;
use bytes::Bytes;
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use opencast_core::{RendererCallback, TransportState};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{debug, info, warn};

/// AirPlay feature flags.
/// Bit 0: Video, Bit 3: VolumeControl, Bit 4: HLS
const AIRPLAY_FEATURES: u64 = 0x19;

/// AirPlay media receiver — accepts video URLs from iOS devices.
pub struct AirPlayReceiver {
    name: String,
    port: u16,
    callback: Arc<dyn RendererCallback>,
    _mdns: Option<ServiceDaemon>,
}

impl AirPlayReceiver {
    pub fn new(name: String, port: u16, callback: Arc<dyn RendererCallback>) -> Self {
        Self {
            name,
            port,
            callback,
            _mdns: None,
        }
    }

    /// Start the AirPlay HTTP server and mDNS advertisement.
    pub async fn start(mut self) -> anyhow::Result<()> {
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], self.port));
        let listener = TcpListener::bind(addr).await?;
        info!("AirPlay receiver '{}' listening on port {}", self.name, self.port);

        // Start mDNS advertisement
        let device_id = get_or_create_device_id();
        self.advertise_mdns(&device_id)?;

        let callback = self.callback.clone();
        let device_id = Arc::new(device_id);

        loop {
            let (stream, peer) = listener.accept().await?;
            debug!("AirPlay connection from {peer}");

            let cb = callback.clone();
            let did = device_id.clone();

            tokio::spawn(async move {
                let service = service_fn(move |req| {
                    let cb = cb.clone();
                    let did = did.clone();
                    async move { handle_request(req, &cb, &did).await }
                });

                if let Err(e) = http1::Builder::new()
                    .serve_connection(TokioIo::new(stream), service)
                    .await
                {
                    if !e.to_string().contains("connection closed") {
                        warn!("AirPlay connection error: {e}");
                    }
                }
            });
        }
    }

    /// Register the AirPlay service via mDNS (Bonjour).
    fn advertise_mdns(&mut self, device_id: &str) -> anyhow::Result<()> {
        let mdns = ServiceDaemon::new()?;

        let mut properties = HashMap::new();
        properties.insert("deviceid".to_string(), device_id.to_string());
        properties.insert("features".to_string(), format!("0x{AIRPLAY_FEATURES:X}"));
        properties.insert("model".to_string(), "OpenCast".to_string());
        properties.insert("srcvers".to_string(), "220.68".to_string());
        properties.insert("vv".to_string(), "2".to_string());

        let service_type = "_airplay._tcp.local.";
        let host = format!("{}.local.", hostname());

        let service = ServiceInfo::new(
            service_type,
            &self.name,
            &host,
            "",
            self.port,
            properties,
        )?;

        mdns.register(service)?;
        info!("AirPlay mDNS advertised as '{}' ({})", self.name, device_id);

        // Store daemon so it stays alive and can be cleanly dropped on shutdown.
        self._mdns = Some(mdns);

        Ok(())
    }
}

/// Route an incoming AirPlay HTTP request.
async fn handle_request(
    req: Request<Incoming>,
    callback: &Arc<dyn RendererCallback>,
    device_id: &str,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("").to_string();

    debug!("AirPlay {method} {path}");

    let response = match (method, path.as_str()) {
        (Method::POST, "/play") => handle_play(req, callback).await,
        (Method::POST, "/rate") => handle_rate(&query, callback),
        (Method::POST, "/scrub") => handle_scrub(&query, callback),
        (Method::POST, "/stop") => handle_stop(callback),
        (Method::GET, "/playback-info") => handle_playback_info(callback),
        (Method::GET, "/server-info") => handle_server_info(device_id),
        (Method::POST, "/reverse") => handle_reverse(),
        (Method::PUT, "/setProperty") => handle_set_property(req, callback).await,
        _ => {
            debug!("AirPlay: unhandled {}", path);
            ok_response("")
        }
    };

    Ok(response)
}

/// POST /play — start playing a media URL.
///
/// Body is `text/parameters` format:
/// ```text
/// Content-Location: http://example.com/video.mp4
/// Start-Position: 0.123456
/// ```
async fn handle_play(
    req: Request<Incoming>,
    callback: &Arc<dyn RendererCallback>,
) -> Response<Full<Bytes>> {
    let body = match read_body(req).await {
        Ok(b) => b,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Failed to read body"),
    };

    let params = parse_text_parameters(&body);
    let url = params.get("Content-Location").cloned().unwrap_or_default();

    if url.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "Missing Content-Location");
    }

    let start_position: f64 = params
        .get("Start-Position")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);

    info!("AirPlay: play {url} (start={start_position:.4})");

    callback.on_set_uri(url, String::new());

    // AirPlay sends Start-Position as a ratio (0.0-1.0).
    // We need to wait for duration to be known, then seek.
    if start_position > 0.001 {
        let cb = callback.clone();
        tokio::spawn(async move {
            // Wait for media to load and duration to become available
            for _ in 0..50 {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                let pos = cb.get_position_info();
                if pos.duration > 0.0 {
                    let seek_to = pos.duration * start_position;
                    info!("AirPlay: deferred seek to {seek_to:.1}s (ratio={start_position:.4})");
                    cb.on_seek(seek_to);
                    break;
                }
            }
        });
    }

    ok_response("")
}

/// POST /rate?value=N — set playback rate (0=pause, 1=play).
fn handle_rate(query: &str, callback: &Arc<dyn RendererCallback>) -> Response<Full<Bytes>> {
    let value = parse_query_param(query, "value")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0);

    if value == 0.0 {
        info!("AirPlay: pause");
        callback.on_pause();
    } else {
        info!("AirPlay: play (rate={value})");
        callback.on_play();
    }
    ok_response("")
}

/// POST /scrub?position=N — seek to position in seconds.
fn handle_scrub(query: &str, callback: &Arc<dyn RendererCallback>) -> Response<Full<Bytes>> {
    let position = parse_query_param(query, "position")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.0);

    info!("AirPlay: seek to {position:.1}s");
    callback.on_seek(position);
    ok_response("")
}

/// POST /stop — stop playback.
fn handle_stop(callback: &Arc<dyn RendererCallback>) -> Response<Full<Bytes>> {
    info!("AirPlay: stop");
    callback.on_stop();
    ok_response("")
}

/// GET /playback-info — return current playback status as XML plist.
fn handle_playback_info(callback: &Arc<dyn RendererCallback>) -> Response<Full<Bytes>> {
    let state = callback.get_transport_state();
    let pos = callback.get_position_info();

    let body = match state {
        TransportState::Playing | TransportState::Paused => {
            let rate = if state == TransportState::Playing {
                1.0
            } else {
                0.0
            };
            plist::build_playback_info_plist(pos.duration, pos.position, rate)
        }
        _ => plist::build_playback_info_not_ready_plist(),
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/x-apple-plist+xml")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// GET /server-info — return device capabilities as XML plist.
fn handle_server_info(device_id: &str) -> Response<Full<Bytes>> {
    let body = plist::build_server_info_plist(device_id, AIRPLAY_FEATURES, "OpenCast");

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/x-apple-plist+xml")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}

/// POST /reverse — reverse HTTP connection for server-initiated events.
/// We accept it but don't use it (needed for iOS to consider us a valid receiver).
fn handle_reverse() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::SWITCHING_PROTOCOLS)
        .header("Upgrade", "PTTH/1.0")
        .header("Connection", "Upgrade")
        .body(Full::new(Bytes::new()))
        .unwrap()
}

/// PUT /setProperty — handle property changes (e.g. volume).
async fn handle_set_property(
    req: Request<Incoming>,
    callback: &Arc<dyn RendererCallback>,
) -> Response<Full<Bytes>> {
    let query = req.uri().query().unwrap_or("").to_string();
    let body = match read_body(req).await {
        Ok(b) => b,
        Err(_) => return error_response(StatusCode::BAD_REQUEST, "Failed to read body"),
    };

    // Volume property: forAirTunes.speaker.volume
    if query.contains("volume") || body.contains("volume") {
        // Try to parse volume from the plist body — look for a <real> value
        if let Some(vol) = extract_real_from_plist(&body) {
            let volume = vol.clamp(0.0, 100.0) as u32;
            info!("AirPlay: set volume to {volume}%");
            callback.on_set_volume(volume);
        }
    }

    ok_response("")
}

// --- Helpers ---

/// Parse a `text/parameters` body into key-value pairs.
fn parse_text_parameters(body: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in body.lines() {
        if let Some((key, value)) = line.split_once(':') {
            map.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    map
}

/// Extract a query parameter value by name.
fn parse_query_param<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    query.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        if k == name {
            Some(v)
        } else {
            None
        }
    })
}

/// Extract the first <real>N</real> value from an Apple plist.
fn extract_real_from_plist(body: &str) -> Option<f64> {
    let start = body.find("<real>")? + 6;
    let end = body[start..].find("</real>")? + start;
    body[start..end].parse().ok()
}

/// Read the full body of an HTTP request as a string.
async fn read_body(req: Request<Incoming>) -> anyhow::Result<String> {
    use http_body_util::BodyExt;
    let bytes = req.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

fn ok_response(body: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

fn error_response(status: StatusCode, msg: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .body(Full::new(Bytes::from(msg.to_string())))
        .unwrap()
}

/// Get or create a persistent device ID.
///
/// Stored in `~/.config/opencast/device-id` so iOS devices recognize us across restarts.
fn get_or_create_device_id() -> String {
    let config_dir = dirs_path().join("opencast");
    let id_file = config_dir.join("device-id");

    // Try to read existing ID
    if let Ok(id) = std::fs::read_to_string(&id_file) {
        let id = id.trim().to_string();
        if !id.is_empty() {
            return id;
        }
    }

    // Generate and persist a new one
    let id = uuid::Uuid::new_v4();
    let bytes = id.as_bytes();
    let device_id = format!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]
    );

    let _ = std::fs::create_dir_all(&config_dir);
    let _ = std::fs::write(&id_file, &device_id);
    info!("Generated new device ID: {device_id} (saved to {id_file:?})");

    device_id
}

/// Get the platform config directory (~/.config on Linux, ~/Library/Application Support on macOS).
fn dirs_path() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".config")
        })
}

/// Get the machine hostname.
fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| {
            std::fs::read_to_string("/etc/hostname")
                .map(|s| s.trim().to_string())
        })
        .unwrap_or_else(|_| "opencast".to_string())
}
