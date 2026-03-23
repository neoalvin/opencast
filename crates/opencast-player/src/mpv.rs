use opencast_core::{MediaInfo, PositionInfo, TransportState, VolumeInfo};
use std::process::Stdio;
use std::sync::Mutex;
use tokio::process::{Child, Command};
use tracing::{info, warn};

/// Media player backed by mpv via IPC.
/// For Phase 1, we use a simple subprocess approach.
pub struct MpvPlayer {
    state: Mutex<PlayerState>,
}

struct PlayerState {
    transport: TransportState,
    position: f64,
    duration: f64,
    volume: f64,
    muted: bool,
    current_url: Option<String>,
    child: Option<Child>,
}

impl MpvPlayer {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(PlayerState {
                transport: TransportState::NoMediaPresent,
                position: 0.0,
                duration: 0.0,
                volume: 1.0,
                muted: false,
                current_url: None,
                child: None,
            }),
        }
    }

    async fn kill_current(&self) {
        let mut state = self.state.lock().unwrap();
        if let Some(ref mut child) = state.child {
            let _ = child.kill().await;
        }
        state.child = None;
    }
}

impl super::Player for MpvPlayer {
    async fn load(&self, media: &MediaInfo) -> anyhow::Result<()> {
        self.kill_current().await;

        info!(url = %media.url, "Loading media");

        let child = Command::new("mpv")
            .arg("--no-terminal")
            .arg("--force-window=yes")
            .arg("--keep-open=no")
            .arg(&media.url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match child {
            Ok(child) => {
                let mut state = self.state.lock().unwrap();
                state.child = Some(child);
                state.current_url = Some(media.url.clone());
                state.transport = TransportState::Playing;
                state.position = 0.0;
                state.duration = media.duration.unwrap_or(0.0);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to start mpv: {e}. Make sure mpv is installed.");
                anyhow::bail!("Failed to start mpv: {e}")
            }
        }
    }

    async fn play(&self) -> anyhow::Result<()> {
        let mut state = self.state.lock().unwrap();
        state.transport = TransportState::Playing;
        Ok(())
    }

    async fn pause(&self) -> anyhow::Result<()> {
        let mut state = self.state.lock().unwrap();
        state.transport = TransportState::Paused;
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        self.kill_current().await;
        let mut state = self.state.lock().unwrap();
        state.transport = TransportState::Stopped;
        state.position = 0.0;
        Ok(())
    }

    async fn seek(&self, position_secs: f64) -> anyhow::Result<()> {
        let mut state = self.state.lock().unwrap();
        state.position = position_secs;
        Ok(())
    }

    fn state(&self) -> TransportState {
        self.state.lock().unwrap().transport
    }

    fn position(&self) -> PositionInfo {
        let state = self.state.lock().unwrap();
        PositionInfo {
            position: state.position,
            duration: state.duration,
            track_uri: state.current_url.clone(),
        }
    }

    fn volume(&self) -> VolumeInfo {
        let state = self.state.lock().unwrap();
        VolumeInfo {
            level: state.volume,
            muted: state.muted,
        }
    }

    async fn set_volume(&self, level: f64) -> anyhow::Result<()> {
        let mut state = self.state.lock().unwrap();
        state.volume = level.clamp(0.0, 1.0);
        Ok(())
    }

    async fn set_mute(&self, muted: bool) -> anyhow::Result<()> {
        let mut state = self.state.lock().unwrap();
        state.muted = muted;
        Ok(())
    }
}
