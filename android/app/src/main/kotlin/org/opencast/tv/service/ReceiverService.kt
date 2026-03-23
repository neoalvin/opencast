package org.opencast.tv.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.net.wifi.WifiManager
import android.os.Binder
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import android.util.Log
import androidx.lifecycle.LifecycleService
import org.opencast.tv.airplay.AirPlayServer
import org.opencast.tv.dlna.DlnaRendererServer
import org.opencast.tv.player.ExoPlayerWrapper

private const val TAG = "ReceiverService"
private const val CHANNEL_ID = "opencast_receiver"
private const val NOTIFICATION_ID = 1

class ReceiverService : LifecycleService() {

    inner class LocalBinder : Binder() {
        val service: ReceiverService get() = this@ReceiverService
    }

    private val binder = LocalBinder()

    var playerWrapper: ExoPlayerWrapper? = null
        private set

    private var dlnaServer: DlnaRendererServer? = null
    private var airPlayServer: AirPlayServer? = null
    private var wifiLock: WifiManager.WifiLock? = null
    private var wakeLock: PowerManager.WakeLock? = null
    private var networkCallback: ConnectivityManager.NetworkCallback? = null

    override fun onBind(intent: Intent): IBinder {
        super.onBind(intent)
        return binder
    }

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
        startForeground(NOTIFICATION_ID, buildNotification())

        acquireLocks()

        // ExoPlayer must be created on the main thread — we're in onCreate so that's fine
        val player = ExoPlayerWrapper(applicationContext)
        playerWrapper = player

        // Start DLNA renderer
        dlnaServer = DlnaRendererServer(
            context = applicationContext,
            callback = player,
            port = 49152,
            friendlyName = "OpenCast TV"
        ).also { it.start() }

        // Start AirPlay receiver
        airPlayServer = AirPlayServer(
            context = applicationContext,
            callback = player,
            port = 7000,
            friendlyName = "OpenCast TV"
        ).also { it.start() }

        Log.i(TAG, "ReceiverService started — DLNA + AirPlay ready")

        registerNetworkCallback()
    }

    override fun onDestroy() {
        unregisterNetworkCallback()
        airPlayServer?.stop()
        airPlayServer = null
        dlnaServer?.stop()
        dlnaServer = null
        playerWrapper?.release()
        playerWrapper = null
        releaseLocks()

        Log.i(TAG, "ReceiverService destroyed")
        super.onDestroy()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        super.onStartCommand(intent, flags, startId)
        return START_STICKY
    }

    private fun registerNetworkCallback() {
        val cm = getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
        val request = NetworkRequest.Builder()
            .addTransportType(NetworkCapabilities.TRANSPORT_WIFI)
            .build()

        val callback = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) {
                Log.i(TAG, "WiFi available — restarting discovery")
                dlnaServer?.restartSsdp()
                airPlayServer?.restartMdns()
            }

            override fun onLost(network: Network) {
                Log.w(TAG, "WiFi lost")
            }
        }
        networkCallback = callback
        cm.registerNetworkCallback(request, callback)
    }

    private fun unregisterNetworkCallback() {
        networkCallback?.let {
            val cm = getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
            try { cm.unregisterNetworkCallback(it) } catch (_: Exception) {}
        }
        networkCallback = null
    }

    @Suppress("DEPRECATION")
    private fun acquireLocks() {
        val wifiManager = applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager
        wifiLock = wifiManager.createWifiLock(
            WifiManager.WIFI_MODE_FULL_HIGH_PERF,
            "opencast:receiver"
        ).apply { acquire() }

        val powerManager = applicationContext.getSystemService(Context.POWER_SERVICE) as PowerManager
        wakeLock = powerManager.newWakeLock(
            PowerManager.PARTIAL_WAKE_LOCK,
            "opencast:receiver"
        ).apply { acquire() }
    }

    private fun releaseLocks() {
        wifiLock?.let {
            if (it.isHeld) it.release()
        }
        wifiLock = null

        wakeLock?.let {
            if (it.isHeld) it.release()
        }
        wakeLock = null
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "OpenCast Receiver",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Keeps the casting receiver active"
            }
            val manager = getSystemService(NotificationManager::class.java)
            manager.createNotificationChannel(channel)
        }
    }

    private fun buildNotification(): Notification {
        return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            Notification.Builder(this, CHANNEL_ID)
                .setContentTitle("OpenCast")
                .setContentText("Receiving — DLNA + AirPlay")
                .setSmallIcon(android.R.drawable.ic_media_play)
                .setOngoing(true)
                .build()
        } else {
            @Suppress("DEPRECATION")
            Notification.Builder(this)
                .setContentTitle("OpenCast")
                .setContentText("Receiving — DLNA + AirPlay")
                .setSmallIcon(android.R.drawable.ic_media_play)
                .setOngoing(true)
                .build()
        }
    }
}
