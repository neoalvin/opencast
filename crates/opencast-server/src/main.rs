use anyhow::Result;
use clap::Parser;
use opencast_core::{PositionInfo, TransportState, VolumeInfo};
use opencast_dlna::dmr::{DlnaRenderer, RendererCallback};
use std::sync::{Arc, Mutex};
use tracing::info;

#[derive(Parser)]
#[command(name = "opencast-server", about = "OpenCast DLNA Media Renderer")]
struct Cli {
    /// Device name shown to controllers
    #[arg(short, long, default_value = "OpenCast TV")]
    name: String,

    /// HTTP server port
    #[arg(short, long, default_value_t = 8200)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,opencast=debug".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    info!("Starting OpenCast Server '{}'", cli.name);
    info!("Listening on port {}", cli.port);
    info!("Other devices on the same network can now cast media to this device.");

    let callback = Arc::new(SimpleCallback::new());
    let renderer = DlnaRenderer::new(cli.name, cli.port, callback);

    renderer.start().await?;

    Ok(())
}

/// Simple callback that logs actions and tracks state.
/// In production, this would integrate with a real media player.
struct SimpleCallback {
    state: Mutex<PlayerState>,
}

struct PlayerState {
    transport: TransportState,
    current_url: Option<String>,
    position: f64,
    duration: f64,
    volume: f64,
    muted: bool,
}

impl SimpleCallback {
    fn new() -> Self {
        Self {
            state: Mutex::new(PlayerState {
                transport: TransportState::NoMediaPresent,
                current_url: None,
                position: 0.0,
                duration: 0.0,
                volume: 0.5,
                muted: false,
            }),
        }
    }
}

impl RendererCallback for SimpleCallback {
    fn on_set_uri(&self, url: String, _metadata: String) {
        info!(">> Media URL set: {url}");
        let mut state = self.state.lock().unwrap();
        state.current_url = Some(url);
        state.transport = TransportState::Stopped;
        state.position = 0.0;
    }

    fn on_play(&self) {
        info!(">> Play");
        self.state.lock().unwrap().transport = TransportState::Playing;
    }

    fn on_pause(&self) {
        info!(">> Pause");
        self.state.lock().unwrap().transport = TransportState::Paused;
    }

    fn on_stop(&self) {
        info!(">> Stop");
        let mut state = self.state.lock().unwrap();
        state.transport = TransportState::Stopped;
        state.position = 0.0;
    }

    fn on_seek(&self, position_secs: f64) {
        info!(">> Seek to {position_secs:.1}s");
        self.state.lock().unwrap().position = position_secs;
    }

    fn on_set_volume(&self, volume: u32) {
        info!(">> Volume: {volume}%");
        self.state.lock().unwrap().volume = volume as f64 / 100.0;
    }

    fn on_set_mute(&self, muted: bool) {
        info!(">> Mute: {muted}");
        self.state.lock().unwrap().muted = muted;
    }

    fn get_position_info(&self) -> PositionInfo {
        let state = self.state.lock().unwrap();
        PositionInfo {
            position: state.position,
            duration: state.duration,
            track_uri: state.current_url.clone(),
        }
    }

    fn get_transport_state(&self) -> TransportState {
        self.state.lock().unwrap().transport
    }

    fn get_volume_info(&self) -> VolumeInfo {
        let state = self.state.lock().unwrap();
        VolumeInfo {
            level: state.volume,
            muted: state.muted,
        }
    }
}
