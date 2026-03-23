use anyhow::Result;
use clap::Parser;
use opencast_core::{MediaInfo, PositionInfo, TransportState, VolumeInfo};
use opencast_dlna::dmr::{DlnaRenderer, RendererCallback};
use opencast_player::{MpvPlayer, Player};
use std::sync::Arc;
use tokio::runtime::Handle;
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

    let player = Arc::new(MpvPlayer::new());
    let callback = Arc::new(MpvCallback::new(player, Handle::current()));
    let renderer = DlnaRenderer::new(cli.name, cli.port, callback);

    renderer.start().await?;

    Ok(())
}

/// Callback that bridges DLNA renderer commands to the mpv player.
struct MpvCallback {
    player: Arc<MpvPlayer>,
    handle: Handle,
}

impl MpvCallback {
    fn new(player: Arc<MpvPlayer>, handle: Handle) -> Self {
        Self { player, handle }
    }

    /// Run an async operation on the player from a sync callback context.
    fn block_on<F: std::future::Future<Output = anyhow::Result<()>> + Send + 'static>(&self, f: F) {
        let handle = self.handle.clone();
        std::thread::spawn(move || {
            handle.block_on(async {
                if let Err(e) = f.await {
                    tracing::error!("Player error: {e}");
                }
            });
        });
    }
}

impl RendererCallback for MpvCallback {
    fn on_set_uri(&self, url: String, _metadata: String) {
        info!(">> Cast: {url}");
        let player = self.player.clone();
        self.block_on(async move {
            let media = MediaInfo::new(&url);
            player.load(&media).await
        });
    }

    fn on_play(&self) {
        info!(">> Play");
        let player = self.player.clone();
        self.block_on(async move { player.play().await });
    }

    fn on_pause(&self) {
        info!(">> Pause");
        let player = self.player.clone();
        self.block_on(async move { player.pause().await });
    }

    fn on_stop(&self) {
        info!(">> Stop");
        let player = self.player.clone();
        self.block_on(async move { player.stop().await });
    }

    fn on_seek(&self, position_secs: f64) {
        info!(">> Seek to {position_secs:.1}s");
        let player = self.player.clone();
        self.block_on(async move { player.seek(position_secs).await });
    }

    fn on_set_volume(&self, volume: u32) {
        info!(">> Volume: {volume}%");
        let player = self.player.clone();
        let level = volume as f64 / 100.0;
        self.block_on(async move { player.set_volume(level).await });
    }

    fn on_set_mute(&self, muted: bool) {
        info!(">> Mute: {muted}");
        let player = self.player.clone();
        self.block_on(async move { player.set_mute(muted).await });
    }

    fn get_position_info(&self) -> PositionInfo {
        self.player.position()
    }

    fn get_transport_state(&self) -> TransportState {
        self.player.state()
    }

    fn get_volume_info(&self) -> VolumeInfo {
        self.player.volume()
    }
}
