/// Generate DIDL-Lite metadata XML for a media item.
pub fn didl_lite_metadata(url: &str, title: &str, mime_type: &str) -> String {
    let class = if mime_type.starts_with("audio/") {
        "object.item.audioItem.musicTrack"
    } else if mime_type.starts_with("image/") {
        "object.item.imageItem.photo"
    } else {
        "object.item.videoItem"
    };

    format!(
        r#"&lt;DIDL-Lite xmlns=&quot;urn:schemas-upnp-org:metadata-1-0/DIDL-Lite/&quot;
             xmlns:dc=&quot;http://purl.org/dc/elements/1.1/&quot;
             xmlns:upnp=&quot;urn:schemas-upnp-org:metadata-1-0/upnp/&quot;&gt;
  &lt;item id=&quot;0&quot; parentID=&quot;-1&quot; restricted=&quot;1&quot;&gt;
    &lt;dc:title&gt;{title}&lt;/dc:title&gt;
    &lt;upnp:class&gt;{class}&lt;/upnp:class&gt;
    &lt;res protocolInfo=&quot;http-get:*:{mime_type}:*&quot;&gt;{url}&lt;/res&gt;
  &lt;/item&gt;
&lt;/DIDL-Lite&gt;"#,
        title = xml_escape(title),
        class = class,
        mime_type = mime_type,
        url = xml_escape(url),
    )
}

/// AVTransport SCPD (Service Control Protocol Description).
pub fn av_transport_scpd() -> &'static str {
    include_str!("templates/AVTransport.xml")
}

/// RenderingControl SCPD.
pub fn rendering_control_scpd() -> &'static str {
    include_str!("templates/RenderingControl.xml")
}

/// ConnectionManager SCPD.
pub fn connection_manager_scpd() -> &'static str {
    include_str!("templates/ConnectionManager.xml")
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Format seconds as HH:MM:SS duration string for DLNA.
pub fn format_duration(secs: f64) -> String {
    let total = secs as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

/// Parse a HH:MM:SS duration string to seconds.
pub fn parse_duration(s: &str) -> f64 {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 3 {
        let h: f64 = parts[0].parse().unwrap_or(0.0);
        let m: f64 = parts[1].parse().unwrap_or(0.0);
        let s: f64 = parts[2].parse().unwrap_or(0.0);
        h * 3600.0 + m * 60.0 + s
    } else {
        0.0
    }
}
