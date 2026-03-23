use serde::{Deserialize, Serialize};

/// Metadata about a media item to be cast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaInfo {
    /// URL of the media content.
    pub url: String,
    /// Title of the media.
    pub title: Option<String>,
    /// MIME type (e.g., "video/mp4", "audio/mp3").
    pub mime_type: Option<String>,
    /// Duration in seconds.
    pub duration: Option<f64>,
    /// Thumbnail / album art URL.
    pub thumbnail_url: Option<String>,
    /// Artist or creator name.
    pub artist: Option<String>,
    /// Album name (for audio).
    pub album: Option<String>,
}

impl MediaInfo {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            title: None,
            mime_type: None,
            duration: None,
            thumbnail_url: None,
            artist: None,
            album: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }
}
