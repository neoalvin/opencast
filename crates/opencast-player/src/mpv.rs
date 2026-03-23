use opencast_core::{MediaInfo, PositionInfo, TransportState, VolumeInfo};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tracing::{debug, info};

/// Media player backed by mpv via JSON IPC protocol.
///
/// mpv exposes a Unix domain socket that accepts JSON commands:
/// - `{"command": ["loadfile", "url"]}` — load media
/// - `{"command": ["set_property", "pause", true]}` — pause
/// - `{"command": ["get_property", "time-pos"]}` — get position
///
/// This gives us full real-time control over playback.
pub struct MpvPlayer {
    ipc_path: PathBuf,
    /// Sync RwLock for state — safe to read from any context (sync or async).
    state: Arc<RwLock<PlayerState>>,
    /// Async Mutex for IPC writer + child process — only used in async context.
    conn: Mutex<MpvConnection>,
    request_id: AtomicU64,
}

struct PlayerState {
    transport: TransportState,
    current_url: Option<String>,
    position: f64,
    duration: f64,
    volume: f64,
    muted: bool,
}

struct MpvConnection {
    child: Option<Child>,
    writer: Option<tokio::io::WriteHalf<UnixStream>>,
}

impl Default for MpvPlayer {
    fn default() -> Self {
        Self::new()
    }
}

impl MpvPlayer {
    pub fn new() -> Self {
        let ipc_path =
            std::env::temp_dir().join(format!("opencast-mpv-{}.sock", std::process::id()));
        Self {
            ipc_path,
            state: Arc::new(RwLock::new(PlayerState {
                transport: TransportState::NoMediaPresent,
                current_url: None,
                position: 0.0,
                duration: 0.0,
                volume: 100.0,
                muted: false,
            })),
            conn: Mutex::new(MpvConnection {
                child: None,
                writer: None,
            }),
            request_id: AtomicU64::new(1),
        }
    }

    /// Start mpv process with IPC socket enabled.
    async fn ensure_mpv_running(&self) -> anyhow::Result<()> {
        let mut conn = self.conn.lock().await;
        if conn.child.is_some() && conn.writer.is_some() {
            return Ok(());
        }

        // Clean up old socket
        let _ = std::fs::remove_file(&self.ipc_path);

        info!("Starting mpv with IPC at {:?}", self.ipc_path);

        let child = Command::new("mpv")
            .arg("--idle=yes")
            .arg("--force-window=yes")
            .arg("--keep-open=yes")
            .arg("--no-terminal")
            .arg(format!(
                "--input-ipc-server={}",
                self.ipc_path.display()
            ))
            .arg("--title=OpenCast Player")
            .spawn();

        match child {
            Ok(child) => {
                conn.child = Some(child);
            }
            Err(e) => {
                anyhow::bail!("Failed to start mpv: {e}. Make sure mpv is installed.");
            }
        }

        // Wait for IPC socket to become available
        for _ in 0..50 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if self.ipc_path.exists() {
                break;
            }
        }

        if !self.ipc_path.exists() {
            anyhow::bail!("mpv IPC socket not created after 5 seconds");
        }

        let stream = UnixStream::connect(&self.ipc_path).await?;
        let (reader, writer) = tokio::io::split(stream);
        conn.writer = Some(writer);

        // Spawn event reader in background
        let state = self.state.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(reader).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Ok(msg) = serde_json::from_str::<Value>(&line) {
                    Self::handle_mpv_event(msg, &state);
                }
            }
            debug!("mpv IPC reader exited");
        });

        // Observe properties for real-time updates
        self.send_command_conn(
            &mut conn,
            &json!({"command": ["observe_property", 1, "time-pos"]}),
        )
        .await?;
        self.send_command_conn(
            &mut conn,
            &json!({"command": ["observe_property", 2, "duration"]}),
        )
        .await?;
        self.send_command_conn(
            &mut conn,
            &json!({"command": ["observe_property", 3, "pause"]}),
        )
        .await?;
        self.send_command_conn(
            &mut conn,
            &json!({"command": ["observe_property", 4, "volume"]}),
        )
        .await?;
        self.send_command_conn(
            &mut conn,
            &json!({"command": ["observe_property", 5, "mute"]}),
        )
        .await?;
        self.send_command_conn(
            &mut conn,
            &json!({"command": ["observe_property", 6, "idle-active"]}),
        )
        .await?;

        info!("mpv IPC connected");
        Ok(())
    }

    fn handle_mpv_event(msg: Value, state: &Arc<RwLock<PlayerState>>) {
        if let Some(event) = msg.get("event").and_then(|e| e.as_str()) {
            match event {
                "end-file" => {
                    let mut s = state.write().unwrap();
                    s.transport = TransportState::Stopped;
                    s.position = 0.0;
                    debug!("mpv: playback ended");
                }
                "start-file" => {
                    state.write().unwrap().transport = TransportState::Transitioning;
                    debug!("mpv: file loading");
                }
                "playback-restart" => {
                    state.write().unwrap().transport = TransportState::Playing;
                    debug!("mpv: playback started");
                }
                _ => {}
            }
        }

        // Handle property changes from observe_property
        if msg.get("event").and_then(|e| e.as_str()) == Some("property-change") {
            let name = msg.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let data = msg.get("data");
            let mut s = state.write().unwrap();
            match name {
                "time-pos" => {
                    if let Some(pos) = data.and_then(|d| d.as_f64()) {
                        s.position = pos;
                    }
                }
                "duration" => {
                    if let Some(dur) = data.and_then(|d| d.as_f64()) {
                        s.duration = dur;
                    }
                }
                "pause" => {
                    if let Some(paused) = data.and_then(|d| d.as_bool()) {
                        if paused && s.transport == TransportState::Playing {
                            s.transport = TransportState::Paused;
                        } else if !paused && s.transport == TransportState::Paused {
                            s.transport = TransportState::Playing;
                        }
                    }
                }
                "volume" => {
                    if let Some(vol) = data.and_then(|d| d.as_f64()) {
                        s.volume = vol;
                    }
                }
                "mute" => {
                    if let Some(muted) = data.and_then(|d| d.as_bool()) {
                        s.muted = muted;
                    }
                }
                "idle-active" => {
                    if let Some(idle) = data.and_then(|d| d.as_bool()) {
                        if idle {
                            s.transport = TransportState::Stopped;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    async fn send_command(&self, cmd: &Value) -> anyhow::Result<()> {
        let mut conn = self.conn.lock().await;
        self.send_command_conn(&mut conn, cmd).await
    }

    async fn send_command_conn(
        &self,
        conn: &mut MpvConnection,
        cmd: &Value,
    ) -> anyhow::Result<()> {
        if let Some(ref mut writer) = conn.writer {
            let mut cmd = cmd.clone();
            let id = self.request_id.fetch_add(1, Ordering::Relaxed);
            cmd.as_object_mut()
                .unwrap()
                .insert("request_id".to_string(), json!(id));

            let mut line = serde_json::to_string(&cmd)?;
            line.push('\n');
            writer.write_all(line.as_bytes()).await?;
            Ok(())
        } else {
            anyhow::bail!("mpv IPC not connected")
        }
    }

    pub async fn kill_mpv(&self) {
        let mut conn = self.conn.lock().await;
        conn.writer = None;
        if let Some(ref mut child) = conn.child {
            let _ = child.kill().await;
        }
        conn.child = None;
        self.state.write().unwrap().transport = TransportState::NoMediaPresent;
        let _ = std::fs::remove_file(&self.ipc_path);
    }
}

impl super::Player for MpvPlayer {
    async fn load(&self, media: &MediaInfo) -> anyhow::Result<()> {
        self.ensure_mpv_running().await?;
        info!(url = %media.url, "Loading media via mpv IPC");

        self.send_command(&json!({"command": ["loadfile", media.url, "replace"]}))
            .await?;

        {
            let mut s = self.state.write().unwrap();
            s.current_url = Some(media.url.clone());
            s.transport = TransportState::Transitioning;
            s.position = 0.0;
        }
        Ok(())
    }

    async fn play(&self) -> anyhow::Result<()> {
        self.send_command(&json!({"command": ["set_property", "pause", false]}))
            .await?;
        self.state.write().unwrap().transport = TransportState::Playing;
        Ok(())
    }

    async fn pause(&self) -> anyhow::Result<()> {
        self.send_command(&json!({"command": ["set_property", "pause", true]}))
            .await?;
        self.state.write().unwrap().transport = TransportState::Paused;
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        self.send_command(&json!({"command": ["stop"]})).await?;
        let mut s = self.state.write().unwrap();
        s.transport = TransportState::Stopped;
        s.position = 0.0;
        Ok(())
    }

    async fn seek(&self, position_secs: f64) -> anyhow::Result<()> {
        self.send_command(&json!({"command": ["seek", position_secs, "absolute"]}))
            .await?;
        Ok(())
    }

    fn state(&self) -> TransportState {
        self.state.read().unwrap().transport
    }

    fn position(&self) -> PositionInfo {
        let s = self.state.read().unwrap();
        PositionInfo {
            position: s.position,
            duration: s.duration,
            track_uri: s.current_url.clone(),
        }
    }

    fn volume(&self) -> VolumeInfo {
        let s = self.state.read().unwrap();
        VolumeInfo {
            level: s.volume / 100.0,
            muted: s.muted,
        }
    }

    async fn set_volume(&self, level: f64) -> anyhow::Result<()> {
        let vol = (level * 100.0).clamp(0.0, 100.0);
        self.send_command(&json!({"command": ["set_property", "volume", vol]}))
            .await?;
        Ok(())
    }

    async fn set_mute(&self, muted: bool) -> anyhow::Result<()> {
        self.send_command(&json!({"command": ["set_property", "mute", muted]}))
            .await?;
        Ok(())
    }
}

impl Drop for MpvPlayer {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.ipc_path);
    }
}
