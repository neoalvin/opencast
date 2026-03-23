mod mpv;

pub use mpv::MpvPlayer;

use opencast_core::{MediaInfo, PositionInfo, TransportState, VolumeInfo};

/// Unified player trait for the receiver side.
/// Wraps platform-specific media players (mpv, gstreamer, etc.)
#[allow(async_fn_in_trait)]
pub trait Player: Send + Sync {
    async fn load(&self, media: &MediaInfo) -> anyhow::Result<()>;
    async fn play(&self) -> anyhow::Result<()>;
    async fn pause(&self) -> anyhow::Result<()>;
    async fn stop(&self) -> anyhow::Result<()>;
    async fn seek(&self, position_secs: f64) -> anyhow::Result<()>;
    fn state(&self) -> TransportState;
    fn position(&self) -> PositionInfo;
    fn volume(&self) -> VolumeInfo;
    async fn set_volume(&self, level: f64) -> anyhow::Result<()>;
    async fn set_mute(&self, muted: bool) -> anyhow::Result<()>;
}
