package org.opencast.tv.airplay

import android.util.Log
import io.ktor.http.*
import io.ktor.server.request.*
import io.ktor.server.response.*
import io.ktor.server.routing.*
import kotlinx.coroutines.*
import org.opencast.tv.core.RendererCallback
import org.opencast.tv.core.TransportState

private const val TAG = "AirPlayRoutes"

fun Routing.airPlayRoutes(callback: RendererCallback, deviceId: String) {

    post("/play") {
        val body = call.receiveText()
        val params = parseTextParameters(body)
        val url = params["Content-Location"] ?: ""

        if (url.isEmpty()) {
            call.respond(HttpStatusCode.BadRequest, "Missing Content-Location")
            return@post
        }

        val startPosition = params["Start-Position"]?.toDoubleOrNull() ?: 0.0
        Log.i(TAG, "AirPlay: play $url (start=${"%.4f".format(startPosition)})")

        callback.onSetUri(url, "")

        // AirPlay sends Start-Position as a ratio (0.0-1.0).
        // Wait for duration to be known, then seek.
        if (startPosition > 0.001) {
            CoroutineScope(Dispatchers.IO).launch {
                for (i in 0 until 50) {
                    delay(200)
                    val pos = callback.getPositionInfo()
                    if (pos.duration > 0.0) {
                        val seekTo = pos.duration * startPosition
                        Log.i(TAG, "AirPlay: deferred seek to ${"%.1f".format(seekTo)}s")
                        callback.onSeek(seekTo)
                        break
                    }
                }
            }
        }

        call.respond(HttpStatusCode.OK, "")
    }

    post("/rate") {
        val value = call.request.queryParameters["value"]?.toDoubleOrNull() ?: 0.0
        if (value == 0.0) {
            Log.i(TAG, "AirPlay: pause")
            callback.onPause()
        } else {
            Log.i(TAG, "AirPlay: play (rate=$value)")
            callback.onPlay()
        }
        call.respond(HttpStatusCode.OK, "")
    }

    post("/scrub") {
        val position = call.request.queryParameters["position"]?.toDoubleOrNull() ?: 0.0
        Log.i(TAG, "AirPlay: seek to ${"%.1f".format(position)}s")
        callback.onSeek(position)
        call.respond(HttpStatusCode.OK, "")
    }

    post("/stop") {
        Log.i(TAG, "AirPlay: stop")
        callback.onStop()
        call.respond(HttpStatusCode.OK, "")
    }

    get("/playback-info") {
        val state = callback.getTransportState()
        val pos = callback.getPositionInfo()

        val body = when (state) {
            TransportState.PLAYING, TransportState.PAUSED -> {
                val rate = if (state == TransportState.PLAYING) 1.0 else 0.0
                PlistBuilder.buildPlaybackInfoPlist(pos.duration, pos.position, rate)
            }
            else -> PlistBuilder.buildPlaybackInfoNotReadyPlist()
        }

        call.respondText(body, ContentType("text", "x-apple-plist+xml"))
    }

    get("/server-info") {
        val body = PlistBuilder.buildServerInfoPlist(deviceId)
        call.respondText(body, ContentType("text", "x-apple-plist+xml"))
    }

    post("/reverse") {
        // Reverse HTTP connection — accept but don't use.
        // Needed for iOS to consider us a valid receiver.
        call.response.header("Upgrade", "PTTH/1.0")
        call.response.header("Connection", "Upgrade")
        call.respond(HttpStatusCode.SwitchingProtocols, "")
    }

    put("/setProperty") {
        val query = call.request.queryString()
        val body = call.receiveText()

        if (query.contains("volume") || body.contains("volume")) {
            extractRealFromPlist(body)?.let { vol ->
                val volume = vol.coerceIn(0.0, 100.0).toInt()
                Log.i(TAG, "AirPlay: set volume to $volume%")
                callback.onSetVolume(volume)
            }
        }

        call.respond(HttpStatusCode.OK, "")
    }
}

private fun parseTextParameters(body: String): Map<String, String> {
    return body.lines()
        .mapNotNull { line ->
            val idx = line.indexOf(':')
            if (idx > 0) {
                line.substring(0, idx).trim() to line.substring(idx + 1).trim()
            } else null
        }
        .toMap()
}

private fun extractRealFromPlist(body: String): Double? {
    val start = body.indexOf("<real>")
    if (start < 0) return null
    val valueStart = start + 6
    val end = body.indexOf("</real>", valueStart)
    if (end < 0) return null
    return body.substring(valueStart, end).toDoubleOrNull()
}
