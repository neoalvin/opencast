use serde::{Deserialize, Serialize};
use url::Url;

/// A discovered device on the network (e.g., a DLNA renderer, AirPlay receiver).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// Unique device identifier (UDN for UPnP).
    pub id: String,
    /// Human-readable device name.
    pub name: String,
    /// Device type (e.g., DLNA renderer, AirPlay receiver).
    pub device_type: DeviceType,
    /// Base URL for device control.
    pub location: Url,
    /// Device manufacturer.
    pub manufacturer: Option<String>,
    /// Device model name.
    pub model_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    /// DLNA/UPnP Media Renderer
    DlnaRenderer,
    /// DLNA/UPnP Media Server
    DlnaServer,
    /// AirPlay receiver
    AirPlay,
    /// Google Cast device
    GoogleCast,
}

impl std::fmt::Display for Device {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({:?}) at {}", self.name, self.device_type, self.location)
    }
}
