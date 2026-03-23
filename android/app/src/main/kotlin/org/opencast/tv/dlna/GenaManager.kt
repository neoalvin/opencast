package org.opencast.tv.dlna

import android.util.Log
import kotlinx.coroutines.*
import okhttp3.MediaType.Companion.toMediaType
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.RequestBody.Companion.toRequestBody
import org.opencast.tv.core.RendererCallback
import org.opencast.tv.core.TransportState
import org.opencast.tv.core.VolumeInfo
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap
import kotlin.math.abs

private const val TAG = "GenaManager"

data class GenaSubscriber(
    val sid: String,
    val callbackUrl: String,
    var timeoutSecs: Long,
    var seq: Int = 0,
    var expiresAt: Long = System.currentTimeMillis() + timeoutSecs * 1000
)

class GenaManager(private val callback: RendererCallback) {

    private val subscribers = ConcurrentHashMap<String, GenaSubscriber>()
    private val httpClient = OkHttpClient()
    private var scope: CoroutineScope? = null

    fun start() {
        scope = CoroutineScope(Dispatchers.IO + SupervisorJob())
        scope?.launch { notifyLoop() }
        Log.i(TAG, "GENA manager started")
    }

    fun stop() {
        scope?.cancel()
        scope = null
        subscribers.clear()
    }

    fun handleSubscribe(headers: Map<String, String>): Pair<Int, Map<String, String>> {
        val sid = headers["SID"] ?: headers["sid"]

        // Renewal
        if (sid != null) {
            val sub = subscribers[sid]
            if (sub != null) {
                sub.timeoutSecs = parseTimeout(headers)
                sub.expiresAt = System.currentTimeMillis() + sub.timeoutSecs * 1000
                Log.i(TAG, "GENA: renewed subscription $sid")
                return 200 to mapOf("SID" to sid, "TIMEOUT" to "Second-${sub.timeoutSecs}")
            }
            return 412 to emptyMap()
        }

        // New subscription
        val callbackUrl = (headers["CALLBACK"] ?: headers["callback"] ?: "")
            .trim('<', '>')

        if (callbackUrl.isEmpty()) {
            return 412 to emptyMap()
        }

        val timeout = parseTimeout(headers)
        val newSid = "uuid:${UUID.randomUUID()}"
        val subscriber = GenaSubscriber(newSid, callbackUrl, timeout)
        subscribers[newSid] = subscriber

        Log.i(TAG, "GENA: new subscription $newSid -> $callbackUrl")

        // Send initial event
        scope?.launch {
            val transport = callback.getTransportState()
            val volume = callback.getVolumeInfo()
            val xml = buildLastChangeXml(transport, volume)
            sendNotify(callbackUrl, newSid, 0, xml)
        }

        return 200 to mapOf("SID" to newSid, "TIMEOUT" to "Second-$timeout")
    }

    fun handleUnsubscribe(headers: Map<String, String>): Int {
        val sid = headers["SID"] ?: headers["sid"]
        if (sid != null) {
            subscribers.remove(sid)
            Log.i(TAG, "GENA: unsubscribed $sid")
        }
        return 200
    }

    private suspend fun notifyLoop() {
        var lastTransport = TransportState.NO_MEDIA_PRESENT
        var lastVolume = VolumeInfo(0.5, false)

        while (currentCoroutineContext().isActive) {
            delay(1000)

            // Remove expired subscriptions
            val now = System.currentTimeMillis()
            val expired = subscribers.filter { it.value.expiresAt < now }
            for ((sid, _) in expired) {
                subscribers.remove(sid)
                Log.i(TAG, "GENA: expired subscription $sid")
            }

            val currentTransport = callback.getTransportState()
            val currentVolume = callback.getVolumeInfo()

            val changed = currentTransport != lastTransport
                    || abs(currentVolume.level - lastVolume.level) > 0.01
                    || currentVolume.muted != lastVolume.muted

            if (!changed) continue

            lastTransport = currentTransport
            lastVolume = currentVolume

            val xml = buildLastChangeXml(currentTransport, currentVolume)

            for ((_, sub) in subscribers) {
                sub.seq++
                val sid = sub.sid
                val url = sub.callbackUrl
                val seq = sub.seq
                scope?.launch { sendNotify(url, sid, seq, xml) }
            }
        }
    }

    private fun buildLastChangeXml(transport: TransportState, volume: VolumeInfo): String {
        val volPct = (volume.level * 100).toInt()
        return XmlTemplates.buildLastChangeXml(transport.dlnaString, volPct, volume.muted)
    }

    private fun sendNotify(callbackUrl: String, sid: String, seq: Int, bodyXml: String) {
        val xml = """<?xml version="1.0" encoding="utf-8"?>
<e:propertyset xmlns:e="urn:schemas-upnp-org:event-1-0">
  <e:property>
    <LastChange>$bodyXml</LastChange>
  </e:property>
</e:propertyset>"""

        try {
            val body = xml.toRequestBody("text/xml; charset=utf-8".toMediaType())
            val request = Request.Builder()
                .url(callbackUrl)
                .method("NOTIFY", body)
                .header("Content-Type", "text/xml; charset=\"utf-8\"")
                .header("NT", "upnp:event")
                .header("NTS", "upnp:propchange")
                .header("SID", sid)
                .header("SEQ", seq.toString())
                .build()

            val response = httpClient.newCall(request).execute()
            Log.d(TAG, "GENA notify to $callbackUrl: ${response.code}")
            response.close()
        } catch (e: Exception) {
            Log.d(TAG, "GENA notify failed to $callbackUrl: ${e.message}")
        }
    }

    private fun parseTimeout(headers: Map<String, String>): Long {
        val timeout = headers["TIMEOUT"] ?: headers["timeout"] ?: ""
        return timeout.removePrefix("Second-").toLongOrNull() ?: 300
    }
}
