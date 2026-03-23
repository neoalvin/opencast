package org.opencast.tv.core

enum class TransportState(val dlnaString: String) {
    STOPPED("STOPPED"),
    PLAYING("PLAYING"),
    PAUSED("PAUSED_PLAYBACK"),
    TRANSITIONING("TRANSITIONING"),
    NO_MEDIA_PRESENT("NO_MEDIA_PRESENT");

    companion object {
        fun fromDlnaString(s: String): TransportState = when (s) {
            "STOPPED" -> STOPPED
            "PLAYING" -> PLAYING
            "PAUSED_PLAYBACK" -> PAUSED
            "TRANSITIONING" -> TRANSITIONING
            else -> NO_MEDIA_PRESENT
        }
    }
}
