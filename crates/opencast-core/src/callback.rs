use crate::{PositionInfo, TransportState, VolumeInfo};

/// Callback for handling media commands from a protocol controller (DLNA, AirPlay, etc.).
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
