package org.opencast.tv.ui.viewmodel

import android.app.Application
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.os.Build
import android.os.IBinder
import androidx.lifecycle.AndroidViewModel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import org.opencast.tv.core.PositionInfo
import org.opencast.tv.core.TransportState
import org.opencast.tv.core.VolumeInfo
import org.opencast.tv.player.ExoPlayerWrapper
import org.opencast.tv.service.ReceiverService

class MainViewModel(application: Application) : AndroidViewModel(application) {

    private val _deviceName = MutableStateFlow("OpenCast TV")
    val deviceName: StateFlow<String> = _deviceName

    private val _localIp = MutableStateFlow("")
    val localIp: StateFlow<String> = _localIp

    private val _playerWrapper = MutableStateFlow<ExoPlayerWrapper?>(null)
    val playerWrapper: StateFlow<ExoPlayerWrapper?> = _playerWrapper

    val transportState: StateFlow<TransportState> get() =
        _playerWrapper.value?.transportState ?: MutableStateFlow(TransportState.NO_MEDIA_PRESENT)

    val positionInfo: StateFlow<PositionInfo> get() =
        _playerWrapper.value?.positionInfo ?: MutableStateFlow(PositionInfo())

    val volumeInfo: StateFlow<VolumeInfo> get() =
        _playerWrapper.value?.volumeInfo ?: MutableStateFlow(VolumeInfo())

    val currentTitle: StateFlow<String?> get() =
        _playerWrapper.value?.currentTitle ?: MutableStateFlow(null)

    val errorMessage: StateFlow<String?> get() =
        _playerWrapper.value?.errorMessage ?: MutableStateFlow(null)

    private var bound = false
    private var service: ReceiverService? = null

    private val connection = object : ServiceConnection {
        override fun onServiceConnected(name: ComponentName?, binder: IBinder?) {
            val localBinder = binder as ReceiverService.LocalBinder
            service = localBinder.service
            _playerWrapper.value = localBinder.service.playerWrapper
        }

        override fun onServiceDisconnected(name: ComponentName?) {
            service = null
            _playerWrapper.value = null
        }
    }

    init {
        // Start and bind to the receiver service
        val ctx = application.applicationContext
        val intent = Intent(ctx, ReceiverService::class.java)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            ctx.startForegroundService(intent)
        } else {
            ctx.startService(intent)
        }
        ctx.bindService(intent, connection, Context.BIND_AUTO_CREATE)
        bound = true
    }

    fun setDeviceName(name: String) { _deviceName.value = name }
    fun setLocalIp(ip: String) { _localIp.value = ip }

    override fun onCleared() {
        super.onCleared()
        if (bound) {
            getApplication<Application>().unbindService(connection)
            bound = false
        }
    }
}
