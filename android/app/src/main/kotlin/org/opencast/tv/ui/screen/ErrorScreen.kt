package org.opencast.tv.ui.screen

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import kotlinx.coroutines.delay
import org.opencast.tv.ui.theme.*

@Composable
fun ErrorScreen(
    message: String,
    onDismiss: () -> Unit
) {
    // Auto-dismiss after 5 seconds and return to idle
    LaunchedEffect(message) {
        delay(5000)
        onDismiss()
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(DarkBackground),
        contentAlignment = Alignment.Center
    ) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.Center
        ) {
            Text(
                text = "Playback Error",
                color = Color(0xFFEF5350),
                fontSize = 28.sp
            )

            Spacer(modifier = Modifier.height(16.dp))

            Text(
                text = message,
                color = TextSecondary,
                fontSize = 16.sp
            )

            Spacer(modifier = Modifier.height(32.dp))

            Text(
                text = "Returning to idle in 5 seconds...",
                color = TextSecondary,
                fontSize = 14.sp
            )
        }
    }
}
