/// Build an Apple XML plist for /server-info response.
pub fn build_server_info_plist(device_id: &str, features: u64, model: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>deviceid</key>
    <string>{device_id}</string>
    <key>features</key>
    <integer>{features}</integer>
    <key>model</key>
    <string>{model}</string>
    <key>protovers</key>
    <string>1.0</string>
    <key>srcvers</key>
    <string>220.68</string>
</dict>
</plist>"#
    )
}

/// Build an Apple XML plist for /playback-info response.
pub fn build_playback_info_plist(duration: f64, position: f64, rate: f64) -> String {
    let ready = if duration > 0.0 { "true" } else { "false" };
    let buffered = duration; // report fully buffered for simplicity

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>duration</key>
    <real>{duration}</real>
    <key>position</key>
    <real>{position}</real>
    <key>rate</key>
    <real>{rate}</real>
    <key>readyToPlay</key>
    <{ready}/>
    <key>playbackBufferedRange</key>
    <dict>
        <key>start</key>
        <real>0</real>
        <key>duration</key>
        <real>{buffered}</real>
    </dict>
    <key>loadedTimeRanges</key>
    <array>
        <dict>
            <key>start</key>
            <real>0</real>
            <key>duration</key>
            <real>{buffered}</real>
        </dict>
    </array>
    <key>seekableTimeRanges</key>
    <array>
        <dict>
            <key>start</key>
            <real>0</real>
            <key>duration</key>
            <real>{duration}</real>
        </dict>
    </array>
</dict>
</plist>"#
    )
}

/// Build a minimal "not yet playing" plist for /playback-info when idle.
pub fn build_playback_info_not_ready_plist() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>readyToPlay</key>
    <false/>
</dict>
</plist>"#
        .to_string()
}
