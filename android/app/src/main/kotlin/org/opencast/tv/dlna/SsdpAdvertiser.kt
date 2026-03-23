package org.opencast.tv.dlna

import android.content.Context
import android.net.wifi.WifiManager
import android.util.Log
import kotlinx.coroutines.*
import java.net.DatagramPacket
import java.net.DatagramSocket
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.MulticastSocket
import java.net.NetworkInterface

private const val TAG = "SsdpAdvertiser"
private const val SSDP_ADDRESS = "239.255.255.250"
private const val SSDP_PORT = 1900

class SsdpAdvertiser(
    private val context: Context,
    private val udn: String,
    private val descriptionUrl: String
) {
    private var scope: CoroutineScope? = null
    private var multicastLock: WifiManager.MulticastLock? = null

    fun start() {
        val wifiManager = context.applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager
        multicastLock = wifiManager.createMulticastLock("opencast-ssdp").apply {
            setReferenceCounted(false)
            acquire()
        }

        scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

        // M-SEARCH responder
        scope?.launch { respondToMSearch() }

        // Periodic NOTIFY alive
        scope?.launch { sendPeriodicNotify() }

        Log.i(TAG, "SSDP advertiser started")
    }

    fun stop() {
        scope?.cancel()
        scope = null
        multicastLock?.release()
        multicastLock = null

        // Send ssdp:byebye
        try {
            val socket = DatagramSocket()
            val dest = InetSocketAddress(InetAddress.getByName(SSDP_ADDRESS), SSDP_PORT)
            val byebye = buildNotify("ssdp:byebye")
            val packet = DatagramPacket(byebye.toByteArray(), byebye.length, dest.address, dest.port)
            socket.send(packet)
            socket.close()
        } catch (e: Exception) {
            Log.w(TAG, "Failed to send byebye: ${e.message}")
        }

        Log.i(TAG, "SSDP advertiser stopped")
    }

    private suspend fun respondToMSearch() {
        try {
            val socket = MulticastSocket(SSDP_PORT)
            socket.reuseAddress = true
            val group = InetAddress.getByName(SSDP_ADDRESS)
            socket.joinGroup(InetSocketAddress(group, SSDP_PORT), NetworkInterface.getByIndex(0))

            val buf = ByteArray(4096)
            while (currentCoroutineContext().isActive) {
                try {
                    val packet = DatagramPacket(buf, buf.size)
                    socket.receive(packet)
                    val msg = String(packet.data, 0, packet.length)

                    if (msg.contains("M-SEARCH") &&
                        (msg.contains("MediaRenderer") || msg.contains("ssdp:all") || msg.contains("upnp:rootdevice"))
                    ) {
                        val response = """HTTP/1.1 200 OK
CACHE-CONTROL: max-age=1800
LOCATION: $descriptionUrl
SERVER: OpenCast/0.1 UPnP/1.0
ST: urn:schemas-upnp-org:device:MediaRenderer:1
USN: uuid:${udn}::urn:schemas-upnp-org:device:MediaRenderer:1

""".replace("\n", "\r\n")

                        val responsePacket = DatagramPacket(
                            response.toByteArray(), response.length,
                            packet.address, packet.port
                        )
                        socket.send(responsePacket)
                        Log.d(TAG, "Responded to M-SEARCH from ${packet.address}")
                    }
                } catch (e: Exception) {
                    if (currentCoroutineContext().isActive) {
                        delay(100)
                    }
                }
            }
            socket.close()
        } catch (e: Exception) {
            Log.e(TAG, "M-SEARCH responder error: ${e.message}")
        }
    }

    private suspend fun sendPeriodicNotify() {
        try {
            val socket = DatagramSocket()
            val dest = InetAddress.getByName(SSDP_ADDRESS)

            while (currentCoroutineContext().isActive) {
                val notify = buildNotify("ssdp:alive")
                val packet = DatagramPacket(notify.toByteArray(), notify.length, dest, SSDP_PORT)
                socket.send(packet)
                Log.d(TAG, "Sent SSDP NOTIFY alive")
                delay(60_000)
            }
            socket.close()
        } catch (e: Exception) {
            Log.e(TAG, "SSDP NOTIFY error: ${e.message}")
        }
    }

    private fun buildNotify(nts: String): String {
        return """NOTIFY * HTTP/1.1
HOST: 239.255.255.250:1900
CACHE-CONTROL: max-age=1800
LOCATION: $descriptionUrl
NT: urn:schemas-upnp-org:device:MediaRenderer:1
NTS: $nts
SERVER: OpenCast/0.1 UPnP/1.0
USN: uuid:${udn}::urn:schemas-upnp-org:device:MediaRenderer:1

""".replace("\n", "\r\n")
    }
}
