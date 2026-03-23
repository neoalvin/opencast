package org.opencast.tv.dlna

import android.content.Context
import android.util.Log
import io.ktor.http.*
import io.ktor.server.engine.*
import io.ktor.server.netty.*
import io.ktor.server.request.*
import io.ktor.server.response.*
import io.ktor.server.routing.*
import org.opencast.tv.R
import org.opencast.tv.core.RendererCallback
import java.net.Inet4Address
import java.net.NetworkInterface
import java.util.UUID

private const val TAG = "DlnaRendererServer"

class DlnaRendererServer(
    private val context: Context,
    private val callback: RendererCallback,
    private val port: Int = 49152,
    private val friendlyName: String = "OpenCast TV"
) {
    private val udn = UUID.randomUUID().toString()
    private var server: EmbeddedServer<NettyApplicationEngine, NettyApplicationEngine.Configuration>? = null
    private var ssdpAdvertiser: SsdpAdvertiser? = null
    private val genaManager = GenaManager(callback)

    fun start() {
        val localIp = getLocalIp()
        val baseUrl = "http://$localIp:$port"
        val descriptionUrl = "$baseUrl/description.xml"

        Log.i(TAG, "DLNA Renderer '$friendlyName' starting on $baseUrl")

        server = embeddedServer(Netty, port = port) {
            routing {
                // Device description
                get("/description.xml") {
                    val xml = XmlTemplates.buildDeviceDescription(friendlyName, udn, baseUrl)
                    call.respondText(xml, ContentType.Text.Xml)
                }

                // SCPD service descriptions
                get("/AVTransport/scpd.xml") {
                    call.respondText(readRawResource(R.raw.av_transport_scpd), ContentType.Text.Xml)
                }
                get("/RenderingControl/scpd.xml") {
                    call.respondText(readRawResource(R.raw.rendering_control_scpd), ContentType.Text.Xml)
                }
                get("/ConnectionManager/scpd.xml") {
                    call.respondText(readRawResource(R.raw.connection_manager_scpd), ContentType.Text.Xml)
                }

                // SOAP control endpoints
                post("/AVTransport/control") {
                    val body = call.receiveText()
                    val xml = SoapHandler.handleAvTransport(body, callback)
                    call.respondText(xml, ContentType.Text.Xml)
                }
                post("/RenderingControl/control") {
                    val body = call.receiveText()
                    val xml = SoapHandler.handleRenderingControl(body, callback)
                    call.respondText(xml, ContentType.Text.Xml)
                }
                post("/ConnectionManager/control") {
                    val body = call.receiveText()
                    val xml = SoapHandler.handleConnectionManager(body)
                    call.respondText(xml, ContentType.Text.Xml)
                }

                // GENA event subscription endpoints
                handleGena("/AVTransport/event")
                handleGena("/RenderingControl/event")
                handleGena("/ConnectionManager/event")
            }
        }.start(wait = false)

        // Start GENA event notifier
        genaManager.start()

        // Start SSDP multicast advertisement
        ssdpAdvertiser = SsdpAdvertiser(context, udn, descriptionUrl).also { it.start() }

        Log.i(TAG, "DLNA Renderer ready")
    }

    fun stop() {
        ssdpAdvertiser?.stop()
        ssdpAdvertiser = null
        genaManager.stop()
        server?.stop(1000, 2000)
        server = null
        Log.i(TAG, "DLNA Renderer stopped")
    }

    fun restartSsdp() {
        ssdpAdvertiser?.stop()
        val localIp = getLocalIp()
        val baseUrl = "http://$localIp:$port"
        val descriptionUrl = "$baseUrl/description.xml"
        ssdpAdvertiser = SsdpAdvertiser(context, udn, descriptionUrl).also { it.start() }
        Log.i(TAG, "SSDP restarted on $baseUrl")
    }

    private fun Routing.handleGena(path: String) {
        route(path) {
            // Ktor doesn't have built-in SUBSCRIBE/UNSUBSCRIBE methods,
            // so we handle all methods and check manually
            handle {
                val method = call.request.httpMethod.value.uppercase()
                val headers = mutableMapOf<String, String>()
                call.request.headers.forEach { name, values ->
                    headers[name] = values.firstOrNull() ?: ""
                }

                when (method) {
                    "SUBSCRIBE" -> {
                        val (status, responseHeaders) = genaManager.handleSubscribe(headers)
                        responseHeaders.forEach { (k, v) ->
                            call.response.header(k, v)
                        }
                        call.respond(HttpStatusCode.fromValue(status), "")
                    }
                    "UNSUBSCRIBE" -> {
                        val status = genaManager.handleUnsubscribe(headers)
                        call.respond(HttpStatusCode.fromValue(status), "")
                    }
                    else -> {
                        call.respond(HttpStatusCode.MethodNotAllowed, "")
                    }
                }
            }
        }
    }

    private fun readRawResource(resId: Int): String {
        return context.resources.openRawResource(resId).bufferedReader().use { it.readText() }
    }

    private fun getLocalIp(): String {
        try {
            for (iface in NetworkInterface.getNetworkInterfaces()) {
                if (iface.isLoopback || !iface.isUp) continue
                for (addr in iface.inetAddresses) {
                    if (addr is Inet4Address && !addr.isLoopbackAddress) {
                        return addr.hostAddress ?: "0.0.0.0"
                    }
                }
            }
        } catch (e: Exception) {
            Log.w(TAG, "Failed to get local IP: ${e.message}")
        }
        return "0.0.0.0"
    }
}
