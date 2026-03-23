package org.opencast.tv.player

import android.content.Context
import android.os.Handler
import android.os.Looper
import android.util.Log
import androidx.annotation.OptIn
import androidx.media3.common.MediaItem
import androidx.media3.common.PlaybackException
import androidx.media3.common.Player
import androidx.media3.common.util.UnstableApi
import androidx.media3.exoplayer.ExoPlayer
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import org.opencast.tv.core.PositionInfo
import org.opencast.tv.core.RendererCallback
import org.opencast.tv.core.TransportState
import org.opencast.tv.core.VolumeInfo

/**
 * ExoPlayer wrapper that implements RendererCallback.
 * Replaces the Rust MpvPlayer — protocol servers call this to control playback.
 *
 * Must be created on the main thread (ExoPlayer requirement).
 */
class ExoPlayerWrapper(context: Context) : RendererCallback {

    companion object {
        private const val TAG = "ExoPlayerWrapper"
    }

    val player: ExoPlayer = ExoPlayer.Builder(context).build()
    private val mainHandler = Handler(Looper.getMainLooper())

    private val _transportState = MutableStateFlow(TransportState.NO_MEDIA_PRESENT)
    val transportState: StateFlow<TransportState> = _transportState

    private val _positionInfo = MutableStateFlow(PositionInfo())
    val positionInfo: StateFlow<PositionInfo> = _positionInfo

    private val _volumeInfo = MutableStateFlow(VolumeInfo())
    val volumeInfo: StateFlow<VolumeInfo> = _volumeInfo

    private val _currentTitle = MutableStateFlow<String?>(null)
    val currentTitle: StateFlow<String?> = _currentTitle

    private val _errorMessage = MutableStateFlow<String?>(null)
    val errorMessage: StateFlow<String?> = _errorMessage

    private var currentUri: String? = null

    init {
        player.addListener(object : Player.Listener {
            override fun onPlaybackStateChanged(playbackState: Int) {
                updateTransportState()
            }

            override fun onIsPlayingChanged(isPlaying: Boolean) {
                updateTransportState()
            }

            @OptIn(UnstableApi::class)
            override fun onPlayerError(error: PlaybackException) {
                Log.e(TAG, "Playback error: ${error.message}")
                _errorMessage.value = error.message ?: "Playback error"
                _transportState.value = TransportState.STOPPED
            }
        })

        // Position polling — update every 500ms
        val positionRunnable = object : Runnable {
            override fun run() {
                if (player.playbackState != Player.STATE_IDLE) {
                    _positionInfo.value = PositionInfo(
                        position = player.currentPosition / 1000.0,
                        duration = player.duration.let { if (it > 0) it / 1000.0 else 0.0 },
                        trackUri = currentUri
                    )
                }
                mainHandler.postDelayed(this, 500)
            }
        }
        mainHandler.postDelayed(positionRunnable, 500)
    }

    private fun updateTransportState() {
        _transportState.value = when {
            player.isPlaying -> TransportState.PLAYING
            player.playbackState == Player.STATE_BUFFERING -> TransportState.TRANSITIONING
            player.playbackState == Player.STATE_READY && !player.playWhenReady -> TransportState.PAUSED
            player.playbackState == Player.STATE_ENDED -> TransportState.STOPPED
            player.playbackState == Player.STATE_IDLE -> TransportState.NO_MEDIA_PRESENT
            else -> TransportState.STOPPED
        }
    }

    // --- RendererCallback implementation ---

    override fun onSetUri(url: String, metadata: String) {
        Log.i(TAG, "Load: $url")
        currentUri = url
        _errorMessage.value = null
        _currentTitle.value = if (metadata.isNotBlank()) metadata else url.substringAfterLast('/')
        mainHandler.post {
            player.setMediaItem(MediaItem.fromUri(url))
            player.prepare()
            player.playWhenReady = true
        }
        _transportState.value = TransportState.TRANSITIONING
    }

    override fun onPlay() {
        Log.i(TAG, "Play")
        mainHandler.post { player.play() }
    }

    override fun onPause() {
        Log.i(TAG, "Pause")
        mainHandler.post { player.pause() }
    }

    override fun onStop() {
        Log.i(TAG, "Stop")
        mainHandler.post { player.stop() }
        _transportState.value = TransportState.STOPPED
        _errorMessage.value = null
        _positionInfo.value = PositionInfo()
        currentUri = null
        _currentTitle.value = null
    }

    override fun onSeek(positionSecs: Double) {
        Log.i(TAG, "Seek to ${positionSecs}s")
        mainHandler.post { player.seekTo((positionSecs * 1000).toLong()) }
    }

    override fun onSetVolume(volume: Int) {
        Log.i(TAG, "Volume: $volume%")
        val level = volume.coerceIn(0, 100) / 100f
        mainHandler.post { player.volume = level }
        _volumeInfo.value = _volumeInfo.value.copy(level = level.toDouble())
    }

    override fun onSetMute(muted: Boolean) {
        Log.i(TAG, "Mute: $muted")
        mainHandler.post { player.volume = if (muted) 0f else (_volumeInfo.value.level.toFloat()) }
        _volumeInfo.value = _volumeInfo.value.copy(muted = muted)
    }

    override fun getPositionInfo(): PositionInfo = _positionInfo.value

    override fun getTransportState(): TransportState = _transportState.value

    override fun getVolumeInfo(): VolumeInfo = _volumeInfo.value

    fun release() {
        mainHandler.post { player.release() }
    }
}
