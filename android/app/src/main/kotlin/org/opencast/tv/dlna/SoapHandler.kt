package org.opencast.tv.dlna

import android.util.Log
import org.opencast.tv.core.RendererCallback

private const val TAG = "SoapHandler"

private const val AV_TRANSPORT_URN = "urn:schemas-upnp-org:service:AVTransport:1"
private const val RENDERING_CONTROL_URN = "urn:schemas-upnp-org:service:RenderingControl:1"
private const val CONNECTION_MANAGER_URN = "urn:schemas-upnp-org:service:ConnectionManager:1"

object SoapHandler {

    fun handleAvTransport(body: String, callback: RendererCallback): String {
        val action = extractSoapAction(body)
        Log.d(TAG, "AVTransport action: $action")

        return when (action) {
            "SetAVTransportURI" -> {
                val uri = extractTagValue(body, "CurrentURI") ?: ""
                val metadata = extractTagValue(body, "CurrentURIMetaData") ?: ""
                Log.i(TAG, "SetAVTransportURI: $uri")
                callback.onSetUri(uri, metadata)
                XmlTemplates.buildSoapResponse("SetAVTransportURI", AV_TRANSPORT_URN, "")
            }
            "Play" -> {
                callback.onPlay()
                XmlTemplates.buildSoapResponse("Play", AV_TRANSPORT_URN, "")
            }
            "Pause" -> {
                callback.onPause()
                XmlTemplates.buildSoapResponse("Pause", AV_TRANSPORT_URN, "")
            }
            "Stop" -> {
                callback.onStop()
                XmlTemplates.buildSoapResponse("Stop", AV_TRANSPORT_URN, "")
            }
            "Seek" -> {
                val target = extractTagValue(body, "Target") ?: "00:00:00"
                val secs = DurationUtil.parseDuration(target)
                callback.onSeek(secs)
                XmlTemplates.buildSoapResponse("Seek", AV_TRANSPORT_URN, "")
            }
            "GetPositionInfo" -> {
                val info = callback.getPositionInfo()
                val responseBody = """
                    <Track>1</Track>
                    <TrackDuration>${DurationUtil.formatDuration(info.duration)}</TrackDuration>
                    <TrackMetaData></TrackMetaData>
                    <TrackURI>${info.trackUri ?: ""}</TrackURI>
                    <RelTime>${DurationUtil.formatDuration(info.position)}</RelTime>
                    <AbsTime>${DurationUtil.formatDuration(info.position)}</AbsTime>
                    <RelCount>0</RelCount>
                    <AbsCount>0</AbsCount>
                """.trimIndent()
                XmlTemplates.buildSoapResponse("GetPositionInfo", AV_TRANSPORT_URN, responseBody)
            }
            "GetTransportInfo" -> {
                val state = callback.getTransportState()
                val responseBody = """
                    <CurrentTransportState>${state.dlnaString}</CurrentTransportState>
                    <CurrentTransportStatus>OK</CurrentTransportStatus>
                    <CurrentSpeed>1</CurrentSpeed>
                """.trimIndent()
                XmlTemplates.buildSoapResponse("GetTransportInfo", AV_TRANSPORT_URN, responseBody)
            }
            "GetMediaInfo" -> {
                val info = callback.getPositionInfo()
                val responseBody = """
                    <NrTracks>1</NrTracks>
                    <MediaDuration>${DurationUtil.formatDuration(info.duration)}</MediaDuration>
                    <CurrentURI>${info.trackUri ?: ""}</CurrentURI>
                    <CurrentURIMetaData></CurrentURIMetaData>
                    <NextURI></NextURI>
                    <NextURIMetaData></NextURIMetaData>
                    <PlayMedium>NETWORK</PlayMedium>
                    <RecordMedium>NOT_IMPLEMENTED</RecordMedium>
                    <WriteStatus>NOT_IMPLEMENTED</WriteStatus>
                """.trimIndent()
                XmlTemplates.buildSoapResponse("GetMediaInfo", AV_TRANSPORT_URN, responseBody)
            }
            "GetTransportSettings" -> {
                XmlTemplates.buildSoapResponse(
                    "GetTransportSettings", AV_TRANSPORT_URN,
                    "<PlayMode>NORMAL</PlayMode><RecQualityMode>NOT_IMPLEMENTED</RecQualityMode>"
                )
            }
            "GetDeviceCapabilities" -> {
                XmlTemplates.buildSoapResponse(
                    "GetDeviceCapabilities", AV_TRANSPORT_URN,
                    "<PlayMedia>NETWORK</PlayMedia><RecMedia>NOT_IMPLEMENTED</RecMedia><RecQualityModes>NOT_IMPLEMENTED</RecQualityModes>"
                )
            }
            "GetCurrentTransportActions" -> {
                XmlTemplates.buildSoapResponse(
                    "GetCurrentTransportActions", AV_TRANSPORT_URN,
                    "<Actions>Play,Pause,Stop,Seek</Actions>"
                )
            }
            else -> {
                Log.w(TAG, "Unhandled AVTransport action: $action")
                XmlTemplates.buildSoapResponse(action, AV_TRANSPORT_URN, "")
            }
        }
    }

    fun handleRenderingControl(body: String, callback: RendererCallback): String {
        val action = extractSoapAction(body)
        Log.d(TAG, "RenderingControl action: $action")

        return when (action) {
            "SetVolume" -> {
                val vol = extractTagValue(body, "DesiredVolume")?.toIntOrNull() ?: 50
                callback.onSetVolume(vol)
                XmlTemplates.buildSoapResponse("SetVolume", RENDERING_CONTROL_URN, "")
            }
            "GetVolume" -> {
                val info = callback.getVolumeInfo()
                val vol = (info.level * 100).toInt()
                XmlTemplates.buildSoapResponse(
                    "GetVolume", RENDERING_CONTROL_URN,
                    "<CurrentVolume>$vol</CurrentVolume>"
                )
            }
            "SetMute" -> {
                val muted = extractTagValue(body, "DesiredMute")?.let {
                    it == "1" || it.equals("true", ignoreCase = true)
                } ?: false
                callback.onSetMute(muted)
                XmlTemplates.buildSoapResponse("SetMute", RENDERING_CONTROL_URN, "")
            }
            "GetMute" -> {
                val info = callback.getVolumeInfo()
                val muted = if (info.muted) "1" else "0"
                XmlTemplates.buildSoapResponse(
                    "GetMute", RENDERING_CONTROL_URN,
                    "<CurrentMute>$muted</CurrentMute>"
                )
            }
            else -> {
                Log.w(TAG, "Unhandled RenderingControl action: $action")
                XmlTemplates.buildSoapResponse(action, RENDERING_CONTROL_URN, "")
            }
        }
    }

    fun handleConnectionManager(body: String): String {
        val action = extractSoapAction(body)
        Log.d(TAG, "ConnectionManager action: $action")

        return when (action) {
            "GetProtocolInfo" -> {
                val sink = listOf(
                    "http-get:*:video/mp4:*",
                    "http-get:*:video/x-matroska:*",
                    "http-get:*:video/webm:*",
                    "http-get:*:video/avi:*",
                    "http-get:*:video/x-flv:*",
                    "http-get:*:video/quicktime:*",
                    "http-get:*:audio/mpeg:*",
                    "http-get:*:audio/mp4:*",
                    "http-get:*:audio/flac:*",
                    "http-get:*:audio/wav:*",
                    "http-get:*:audio/x-wav:*",
                    "http-get:*:audio/ogg:*",
                    "http-get:*:image/jpeg:*",
                    "http-get:*:image/png:*",
                    "http-get:*:application/vnd.apple.mpegurl:*",
                    "http-get:*:application/x-mpegURL:*"
                ).joinToString(",")
                XmlTemplates.buildSoapResponse(
                    "GetProtocolInfo", CONNECTION_MANAGER_URN,
                    "<Source></Source><Sink>$sink</Sink>"
                )
            }
            "GetCurrentConnectionIDs" -> {
                XmlTemplates.buildSoapResponse(
                    "GetCurrentConnectionIDs", CONNECTION_MANAGER_URN,
                    "<ConnectionIDs>0</ConnectionIDs>"
                )
            }
            "GetCurrentConnectionInfo" -> {
                XmlTemplates.buildSoapResponse(
                    "GetCurrentConnectionInfo", CONNECTION_MANAGER_URN,
                    "<RcsID>0</RcsID><AVTransportID>0</AVTransportID><ProtocolInfo></ProtocolInfo><PeerConnectionManager></PeerConnectionManager><PeerConnectionID>-1</PeerConnectionID><Direction>Input</Direction><Status>OK</Status>"
                )
            }
            else -> XmlTemplates.buildSoapResponse(action, CONNECTION_MANAGER_URN, "")
        }
    }

    private fun extractSoapAction(body: String): String {
        val pos = body.indexOf("<u:")
        if (pos >= 0) {
            val rest = body.substring(pos + 3)
            val end = rest.indexOfFirst { it == ' ' || it == '>' || it == '/' }
            if (end >= 0) return rest.substring(0, end)
        }
        return "Unknown"
    }

    private fun extractTagValue(body: String, tag: String): String? {
        // Try plain tag
        val open = "<$tag>"
        val close = "</$tag>"
        var start = body.indexOf(open)
        if (start >= 0) {
            val valueStart = start + open.length
            val end = body.indexOf(close, valueStart)
            if (end >= 0) return body.substring(valueStart, end)
        }
        // Try with namespace prefixes
        for (prefix in listOf("u:", "m:")) {
            val openNs = "<$prefix$tag>"
            val closeNs = "</$prefix$tag>"
            start = body.indexOf(openNs)
            if (start >= 0) {
                val valueStart = start + openNs.length
                val end = body.indexOf(closeNs, valueStart)
                if (end >= 0) return body.substring(valueStart, end)
            }
        }
        return null
    }
}
