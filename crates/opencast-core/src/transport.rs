use serde::{Deserialize, Serialize};

/// Current transport state of the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransportState {
    Stopped,
    Playing,
    Paused,
    Transitioning,
    NoMediaPresent,
}

impl std::fmt::Display for TransportState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stopped => write!(f, "STOPPED"),
            Self::Playing => write!(f, "PLAYING"),
            Self::Paused => write!(f, "PAUSED_PLAYBACK"),
            Self::Transitioning => write!(f, "TRANSITIONING"),
            Self::NoMediaPresent => write!(f, "NO_MEDIA_PRESENT"),
        }
    }
}

impl TransportState {
    pub fn from_dlna_str(s: &str) -> Self {
        match s {
            "STOPPED" => Self::Stopped,
            "PLAYING" => Self::Playing,
            "PAUSED_PLAYBACK" => Self::Paused,
            "TRANSITIONING" => Self::Transitioning,
            _ => Self::NoMediaPresent,
        }
    }
}

/// Playback position and duration information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionInfo {
    /// Current playback position in seconds.
    pub position: f64,
    /// Total duration in seconds (0 if unknown).
    pub duration: f64,
    /// URI of the currently playing media.
    pub track_uri: Option<String>,
}

/// Volume information.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct VolumeInfo {
    /// Volume level (0.0 - 1.0).
    pub level: f64,
    /// Whether audio is muted.
    pub muted: bool,
}
