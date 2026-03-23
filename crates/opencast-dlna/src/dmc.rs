use crate::soap;
use crate::xml_templates;
use opencast_core::{Device, MediaInfo, PositionInfo, TransportState, VolumeInfo};
use reqwest::Client;
use tracing::{debug, info};
use url::Url;

const AV_TRANSPORT_SERVICE: &str = "urn:schemas-upnp-org:service:AVTransport:1";
const RENDERING_CONTROL_SERVICE: &str = "urn:schemas-upnp-org:service:RenderingControl:1";

/// DLNA Digital Media Controller (DMC).
/// Controls a remote DLNA Media Renderer by sending SOAP commands.
/// This is the "sender" side — runs on the phone/PC.
pub struct DlnaController {
    client: Client,
    device: Device,
    av_transport_url: String,
    rendering_control_url: String,
}

impl DlnaController {
    /// Create a new controller targeting a specific DLNA renderer device.
    pub fn new(device: Device) -> anyhow::Result<Self> {
        let base = device.location.clone();
        let av_transport_url = Self::resolve_url(&base, "/AVTransport/control");
        let rendering_control_url = Self::resolve_url(&base, "/RenderingControl/control");

        info!(
            "DlnaController targeting {} at {}",
            device.name, device.location
        );

        Ok(Self {
            client: Client::new(),
            device,
            av_transport_url,
            rendering_control_url,
        })
    }

    /// Get the target device.
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Push a media URL to the renderer and start playback.
    /// This is the core "media casting" operation — the renderer fetches
    /// the media directly, and the phone remains free.
    pub async fn cast(&self, media: &MediaInfo) -> anyhow::Result<()> {
        self.set_av_transport_uri(media).await?;
        self.play().await?;
        info!(url = %media.url, "Casting media to {}", self.device.name);
        Ok(())
    }

    /// Set the media URI on the renderer (SetAVTransportURI).
    pub async fn set_av_transport_uri(&self, media: &MediaInfo) -> anyhow::Result<()> {
        let title = media.title.as_deref().unwrap_or("Unknown");
        let mime_type = media.mime_type.as_deref().unwrap_or("video/mp4");
        let metadata = xml_templates::didl_lite_metadata(&media.url, title, mime_type);

        let body = format!(
            "<InstanceID>0</InstanceID>\
             <CurrentURI>{uri}</CurrentURI>\
             <CurrentURIMetaData>{metadata}</CurrentURIMetaData>",
            uri = xml_escape(&media.url),
            metadata = metadata,
        );

        soap::soap_action(
            &self.client,
            &self.av_transport_url,
            AV_TRANSPORT_SERVICE,
            "SetAVTransportURI",
            &body,
        )
        .await?;

        debug!("SetAVTransportURI sent successfully");
        Ok(())
    }

    /// Start/resume playback.
    pub async fn play(&self) -> anyhow::Result<()> {
        soap::soap_action(
            &self.client,
            &self.av_transport_url,
            AV_TRANSPORT_SERVICE,
            "Play",
            "<InstanceID>0</InstanceID><Speed>1</Speed>",
        )
        .await?;
        Ok(())
    }

    /// Pause playback.
    pub async fn pause(&self) -> anyhow::Result<()> {
        soap::soap_action(
            &self.client,
            &self.av_transport_url,
            AV_TRANSPORT_SERVICE,
            "Pause",
            "<InstanceID>0</InstanceID>",
        )
        .await?;
        Ok(())
    }

    /// Stop playback.
    pub async fn stop(&self) -> anyhow::Result<()> {
        soap::soap_action(
            &self.client,
            &self.av_transport_url,
            AV_TRANSPORT_SERVICE,
            "Stop",
            "<InstanceID>0</InstanceID>",
        )
        .await?;
        Ok(())
    }

    /// Seek to position (in seconds).
    pub async fn seek(&self, position_secs: f64) -> anyhow::Result<()> {
        let target = xml_templates::format_duration(position_secs);
        let body = format!(
            "<InstanceID>0</InstanceID>\
             <Unit>REL_TIME</Unit>\
             <Target>{target}</Target>"
        );

        soap::soap_action(
            &self.client,
            &self.av_transport_url,
            AV_TRANSPORT_SERVICE,
            "Seek",
            &body,
        )
        .await?;
        Ok(())
    }

    /// Get current transport state.
    pub async fn get_transport_state(&self) -> anyhow::Result<TransportState> {
        let response = soap::soap_action(
            &self.client,
            &self.av_transport_url,
            AV_TRANSPORT_SERVICE,
            "GetTransportInfo",
            "<InstanceID>0</InstanceID>",
        )
        .await?;

        let state_str = soap::extract_xml_value(&response, "CurrentTransportState")
            .unwrap_or_else(|| "NO_MEDIA_PRESENT".to_string());

        Ok(TransportState::from_dlna_str(&state_str))
    }

    /// Get current position info.
    pub async fn get_position_info(&self) -> anyhow::Result<PositionInfo> {
        let response = soap::soap_action(
            &self.client,
            &self.av_transport_url,
            AV_TRANSPORT_SERVICE,
            "GetPositionInfo",
            "<InstanceID>0</InstanceID>",
        )
        .await?;

        let rel_time = soap::extract_xml_value(&response, "RelTime")
            .unwrap_or_else(|| "00:00:00".to_string());
        let duration = soap::extract_xml_value(&response, "TrackDuration")
            .unwrap_or_else(|| "00:00:00".to_string());
        let track_uri = soap::extract_xml_value(&response, "TrackURI");

        Ok(PositionInfo {
            position: xml_templates::parse_duration(&rel_time),
            duration: xml_templates::parse_duration(&duration),
            track_uri,
        })
    }

    /// Set volume (0-100).
    pub async fn set_volume(&self, volume: u32) -> anyhow::Result<()> {
        let body = format!(
            "<InstanceID>0</InstanceID>\
             <Channel>Master</Channel>\
             <DesiredVolume>{}</DesiredVolume>",
            volume.min(100)
        );

        soap::soap_action(
            &self.client,
            &self.rendering_control_url,
            RENDERING_CONTROL_SERVICE,
            "SetVolume",
            &body,
        )
        .await?;
        Ok(())
    }

    /// Get current volume.
    pub async fn get_volume(&self) -> anyhow::Result<VolumeInfo> {
        let response = soap::soap_action(
            &self.client,
            &self.rendering_control_url,
            RENDERING_CONTROL_SERVICE,
            "GetVolume",
            "<InstanceID>0</InstanceID><Channel>Master</Channel>",
        )
        .await?;

        let volume: f64 = soap::extract_xml_value(&response, "CurrentVolume")
            .and_then(|v| v.parse().ok())
            .unwrap_or(50.0);

        let mute_response = soap::soap_action(
            &self.client,
            &self.rendering_control_url,
            RENDERING_CONTROL_SERVICE,
            "GetMute",
            "<InstanceID>0</InstanceID><Channel>Master</Channel>",
        )
        .await
        .ok();

        let muted = mute_response
            .and_then(|r| soap::extract_xml_value(&r, "CurrentMute"))
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        Ok(VolumeInfo {
            level: volume / 100.0,
            muted,
        })
    }

    fn resolve_url(base: &Url, path: &str) -> String {
        let mut url = base.clone();
        url.set_path(path);
        url.to_string()
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
