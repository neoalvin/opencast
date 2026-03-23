package org.opencast.tv.core

data class PositionInfo(
    val position: Double = 0.0,
    val duration: Double = 0.0,
    val trackUri: String? = null
)
