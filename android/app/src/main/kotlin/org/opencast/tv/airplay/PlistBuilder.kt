package org.opencast.tv.airplay

object PlistBuilder {

    fun buildServerInfoPlist(deviceId: String, features: Long = 0x19, model: String = "OpenCast"): String {
        return """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>deviceid</key>
    <string>$deviceId</string>
    <key>features</key>
    <integer>$features</integer>
    <key>model</key>
    <string>$model</string>
    <key>protovers</key>
    <string>1.0</string>
    <key>srcvers</key>
    <string>220.68</string>
</dict>
</plist>"""
    }

    fun buildPlaybackInfoPlist(duration: Double, position: Double, rate: Double): String {
        val ready = if (duration > 0.0) "true" else "false"
        val buffered = duration
        return """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>duration</key>
    <real>$duration</real>
    <key>position</key>
    <real>$position</real>
    <key>rate</key>
    <real>$rate</real>
    <key>readyToPlay</key>
    <$ready/>
    <key>playbackBufferedRange</key>
    <dict>
        <key>start</key>
        <real>0</real>
        <key>duration</key>
        <real>$buffered</real>
    </dict>
    <key>loadedTimeRanges</key>
    <array>
        <dict>
            <key>start</key>
            <real>0</real>
            <key>duration</key>
            <real>$buffered</real>
        </dict>
    </array>
    <key>seekableTimeRanges</key>
    <array>
        <dict>
            <key>start</key>
            <real>0</real>
            <key>duration</key>
            <real>$duration</real>
        </dict>
    </array>
</dict>
</plist>"""
    }

    fun buildPlaybackInfoNotReadyPlist(): String {
        return """<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>readyToPlay</key>
    <false/>
</dict>
</plist>"""
    }
}
