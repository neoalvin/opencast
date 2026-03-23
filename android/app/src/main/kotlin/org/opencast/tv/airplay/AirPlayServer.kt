package org.opencast.tv.airplay

import android.content.Context
import android.net.nsd.NsdManager
import android.net.nsd.NsdServiceInfo
import android.util.Log
import io.ktor.server.engine.*
import io.ktor.server.netty.*
import io.ktor.server.routing.*
import org.opencast.tv.core.RendererCallback
import java.util.UUID

private const val TAG = "AirPlayServer"
private const val AIRPLAY_FEATURES = 0x19L

class AirPlayServer(
    private val context: Context,
    private val callback: RendererCallback,
    private val port: Int = 7000,
    private val friendlyName: String = "OpenCast TV"
) {
    private var server: EmbeddedServer<NettyApplicationEngine, NettyApplicationEngine.Configuration>? = null
    private var nsdManager: NsdManager? = null
    private var registrationListener: NsdManager.RegistrationListener? = null
    private val deviceId = getOrCreateDeviceId()

    fun start() {
        Log.i(TAG, "AirPlay receiver '$friendlyName' starting on port $port")

        server = embeddedServer(Netty, port = port) {
            routing {
                airPlayRoutes(callback, deviceId)
            }
        }.start(wait = false)

        advertiseMdns()

        Log.i(TAG, "AirPlay receiver ready")
    }

    fun stop() {
        unregisterMdns()
        server?.stop(1000, 2000)
        server = null
        Log.i(TAG, "AirPlay receiver stopped")
    }

    fun restartMdns() {
        unregisterMdns()
        advertiseMdns()
        Log.i(TAG, "AirPlay mDNS re-registered")
    }

    private fun advertiseMdns() {
        val nsd = context.getSystemService(Context.NSD_SERVICE) as NsdManager
        nsdManager = nsd

        val serviceInfo = NsdServiceInfo().apply {
            serviceName = friendlyName
            serviceType = "_airplay._tcp."
            setPort(this@AirPlayServer.port)
            setAttribute("deviceid", deviceId)
            setAttribute("features", "0x${AIRPLAY_FEATURES.toString(16).uppercase()}")
            setAttribute("model", "OpenCast")
            setAttribute("srcvers", "220.68")
            setAttribute("vv", "2")
        }

        registrationListener = object : NsdManager.RegistrationListener {
            override fun onServiceRegistered(info: NsdServiceInfo) {
                Log.i(TAG, "AirPlay mDNS advertised as '${info.serviceName}' ($deviceId)")
            }

            override fun onRegistrationFailed(info: NsdServiceInfo, errorCode: Int) {
                Log.e(TAG, "AirPlay mDNS registration failed: error $errorCode")
            }

            override fun onServiceUnregistered(info: NsdServiceInfo) {
                Log.i(TAG, "AirPlay mDNS unregistered")
            }

            override fun onUnregistrationFailed(info: NsdServiceInfo, errorCode: Int) {
                Log.w(TAG, "AirPlay mDNS unregistration failed: error $errorCode")
            }
        }

        nsd.registerService(serviceInfo, NsdManager.PROTOCOL_DNS_SD, registrationListener)
    }

    private fun unregisterMdns() {
        registrationListener?.let { listener ->
            try {
                nsdManager?.unregisterService(listener)
            } catch (e: Exception) {
                Log.w(TAG, "Failed to unregister mDNS: ${e.message}")
            }
        }
        registrationListener = null
        nsdManager = null
    }

    private fun getOrCreateDeviceId(): String {
        val prefs = context.getSharedPreferences("opencast", Context.MODE_PRIVATE)
        val existing = prefs.getString("airplay_device_id", null)
        if (!existing.isNullOrBlank()) return existing

        val uuid = UUID.randomUUID()
        val bytes = uuid.toString().replace("-", "")
        val id = "${bytes.substring(0, 2)}:${bytes.substring(2, 4)}:${bytes.substring(4, 6)}:" +
                "${bytes.substring(6, 8)}:${bytes.substring(8, 10)}:${bytes.substring(10, 12)}"
        val deviceId = id.uppercase()

        prefs.edit().putString("airplay_device_id", deviceId).apply()
        Log.i(TAG, "Generated new AirPlay device ID: $deviceId")
        return deviceId
    }
}
