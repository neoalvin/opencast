package org.opencast.tv.core

interface RendererCallback {
    fun onSetUri(url: String, metadata: String)
    fun onPlay()
    fun onPause()
    fun onStop()
    fun onSeek(positionSecs: Double)
    fun onSetVolume(volume: Int)
    fun onSetMute(muted: Boolean)
    fun getPositionInfo(): PositionInfo
    fun getTransportState(): TransportState
    fun getVolumeInfo(): VolumeInfo
}
