package org.opencast.tv.ui.screen

import android.view.KeyEvent
import androidx.compose.foundation.background
import androidx.compose.foundation.focusable
import androidx.compose.foundation.layout.*
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.key.onKeyEvent
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.ui.PlayerView
import kotlinx.coroutines.delay
import org.opencast.tv.core.PositionInfo
import org.opencast.tv.core.TransportState
import org.opencast.tv.ui.theme.*

@Composable
fun PlayerScreen(
    player: ExoPlayer,
    title: String?,
    positionInfo: PositionInfo,
    transportState: TransportState,
    onPlayPause: () -> Unit,
    onSeek: (Double) -> Unit,
    onStop: () -> Unit
) {
    var showOverlay by remember { mutableStateOf(true) }
    val focusRequester = remember { FocusRequester() }

    // Auto-hide overlay after 5 seconds
    LaunchedEffect(showOverlay) {
        if (showOverlay) {
            delay(5000)
            showOverlay = false
        }
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Color.Black)
            .focusRequester(focusRequester)
            .focusable()
            .onKeyEvent { event ->
                if (event.nativeKeyEvent.action != KeyEvent.ACTION_DOWN) return@onKeyEvent false
                when (event.nativeKeyEvent.keyCode) {
                    KeyEvent.KEYCODE_DPAD_CENTER, KeyEvent.KEYCODE_ENTER -> {
                        if (showOverlay) onPlayPause() else showOverlay = true
                        true
                    }
                    KeyEvent.KEYCODE_DPAD_LEFT -> {
                        val seekTo = (positionInfo.position - 10).coerceAtLeast(0.0)
                        onSeek(seekTo)
                        showOverlay = true
                        true
                    }
                    KeyEvent.KEYCODE_DPAD_RIGHT -> {
                        val seekTo = (positionInfo.position + 10).coerceAtMost(positionInfo.duration)
                        onSeek(seekTo)
                        showOverlay = true
                        true
                    }
                    KeyEvent.KEYCODE_BACK -> {
                        onStop()
                        true
                    }
                    else -> {
                        showOverlay = true
                        false
                    }
                }
            }
    ) {
        // ExoPlayer video surface
        AndroidView(
            factory = { ctx ->
                PlayerView(ctx).apply {
                    this.player = player
                    useController = false
                }
            },
            modifier = Modifier.fillMaxSize()
        )

        // Overlay
        if (showOverlay) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .background(Color.Black.copy(alpha = 0.5f))
                    .padding(32.dp),
                verticalArrangement = Arrangement.Bottom
            ) {
                title?.let {
                    Text(
                        text = it,
                        color = TextPrimary,
                        fontSize = 20.sp,
                        modifier = Modifier.padding(bottom = 12.dp)
                    )
                }

                // Progress bar
                if (positionInfo.duration > 0) {
                    val progress = (positionInfo.position / positionInfo.duration).toFloat()
                        .coerceIn(0f, 1f)
                    LinearProgressIndicator(
                        progress = { progress },
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(4.dp),
                        color = PrimaryBlue,
                        trackColor = Color.White.copy(alpha = 0.3f)
                    )

                    Spacer(modifier = Modifier.height(8.dp))

                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Text(
                            text = formatTime(positionInfo.position),
                            color = TextSecondary,
                            fontSize = 14.sp
                        )
                        Text(
                            text = if (transportState == TransportState.PAUSED) "已暂停" else "",
                            color = TextPrimary,
                            fontSize = 14.sp
                        )
                        Text(
                            text = formatTime(positionInfo.duration),
                            color = TextSecondary,
                            fontSize = 14.sp
                        )
                    }
                }
            }
        }
    }

    LaunchedEffect(Unit) {
        focusRequester.requestFocus()
    }
}

private fun formatTime(secs: Double): String {
    val total = secs.toInt()
    val h = total / 3600
    val m = (total % 3600) / 60
    val s = total % 60
    return if (h > 0) String.format("%d:%02d:%02d", h, m, s)
    else String.format("%02d:%02d", m, s)
}
