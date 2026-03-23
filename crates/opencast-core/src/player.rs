use crate::{MediaInfo, PositionInfo, TransportState, VolumeInfo};

/// Trait for controlling media playback on a renderer.
#[allow(async_fn_in_trait)]
pub trait PlaybackControl {
    type Error: std::error::Error + Send + Sync;

    /// Set media URI and start loading.
    async fn set_media(&self, media: &MediaInfo) -> Result<(), Self::Error>;

    /// Start or resume playback.
    async fn play(&self) -> Result<(), Self::Error>;

    /// Pause playback.
    async fn pause(&self) -> Result<(), Self::Error>;

    /// Stop playback.
    async fn stop(&self) -> Result<(), Self::Error>;

    /// Seek to a position in seconds.
    async fn seek(&self, position_secs: f64) -> Result<(), Self::Error>;

    /// Get current transport state.
    async fn get_transport_state(&self) -> Result<TransportState, Self::Error>;

    /// Get current playback position info.
    async fn get_position_info(&self) -> Result<PositionInfo, Self::Error>;

    /// Set volume (0.0 - 1.0).
    async fn set_volume(&self, level: f64) -> Result<(), Self::Error>;

    /// Get volume info.
    async fn get_volume(&self) -> Result<VolumeInfo, Self::Error>;

    /// Set mute state.
    async fn set_mute(&self, muted: bool) -> Result<(), Self::Error>;
}

/// Trait for a media renderer that can receive and play media.
/// This is implemented by the TV-side receiver.
#[allow(async_fn_in_trait)]
pub trait MediaRenderer {
    type Error: std::error::Error + Send + Sync;

    /// Handle a request to play a new media item.
    async fn on_set_media(&self, media: &MediaInfo) -> Result<(), Self::Error>;

    /// Handle play command.
    async fn on_play(&self) -> Result<(), Self::Error>;

    /// Handle pause command.
    async fn on_pause(&self) -> Result<(), Self::Error>;

    /// Handle stop command.
    async fn on_stop(&self) -> Result<(), Self::Error>;

    /// Handle seek command.
    async fn on_seek(&self, position_secs: f64) -> Result<(), Self::Error>;

    /// Get current transport state.
    fn transport_state(&self) -> TransportState;

    /// Get current position info.
    fn position_info(&self) -> PositionInfo;

    /// Get current volume info.
    fn volume_info(&self) -> VolumeInfo;

    /// Set volume.
    async fn on_set_volume(&self, level: f64) -> Result<(), Self::Error>;

    /// Set mute.
    async fn on_set_mute(&self, muted: bool) -> Result<(), Self::Error>;
}
