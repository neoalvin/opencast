package org.opencast.tv

import android.net.wifi.WifiManager
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.lifecycle.ViewModelProvider
import org.opencast.tv.core.TransportState
import org.opencast.tv.ui.screen.ErrorScreen
import org.opencast.tv.ui.screen.IdleScreen
import org.opencast.tv.ui.screen.PlayerScreen
import org.opencast.tv.ui.viewmodel.MainViewModel

class MainActivity : ComponentActivity() {

    private lateinit var viewModel: MainViewModel

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        viewModel = ViewModelProvider(this)[MainViewModel::class.java]
        viewModel.setLocalIp(getLocalIpAddress())
        viewModel.setDeviceName("OpenCast TV")

        setContent {
            val wrapper by viewModel.playerWrapper.collectAsState()
            val state by viewModel.transportState.collectAsState()
            val position by viewModel.positionInfo.collectAsState()
            val title by viewModel.currentTitle.collectAsState()
            val deviceName by viewModel.deviceName.collectAsState()
            val localIp by viewModel.localIp.collectAsState()
            val error by viewModel.errorMessage.collectAsState()

            val player = wrapper

            when {
                error != null -> {
                    ErrorScreen(
                        message = error ?: "Unknown error",
                        onDismiss = { player?.onStop() }
                    )
                }
                player != null && state in listOf(
                    TransportState.PLAYING,
                    TransportState.PAUSED,
                    TransportState.TRANSITIONING
                ) -> {
                    PlayerScreen(
                        player = player.player,
                        title = title,
                        positionInfo = position,
                        transportState = state,
                        onPlayPause = {
                            if (state == TransportState.PLAYING) {
                                player.onPause()
                            } else {
                                player.onPlay()
                            }
                        },
                        onSeek = { player.onSeek(it) },
                        onStop = { player.onStop() }
                    )
                }
                else -> {
                    IdleScreen(
                        deviceName = deviceName,
                        localIp = localIp
                    )
                }
            }
        }
    }

    @Suppress("DEPRECATION")
    private fun getLocalIpAddress(): String {
        val wifiManager = applicationContext.getSystemService(WIFI_SERVICE) as WifiManager
        val ip = wifiManager.connectionInfo.ipAddress
        if (ip == 0) return ""
        return String.format(
            "%d.%d.%d.%d",
            ip and 0xff, ip shr 8 and 0xff, ip shr 16 and 0xff, ip shr 24 and 0xff
        )
    }
}
