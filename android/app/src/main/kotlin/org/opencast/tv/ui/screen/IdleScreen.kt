package org.opencast.tv.ui.screen

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import org.opencast.tv.ui.theme.*

@Composable
fun IdleScreen(
    deviceName: String,
    localIp: String,
    dlnaPort: Int = 49152,
    airplayPort: Int = 7000
) {
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
                text = "OpenCast",
                color = PrimaryBlue,
                fontSize = 48.sp,
                fontWeight = FontWeight.Bold
            )

            Spacer(modifier = Modifier.height(16.dp))

            Text(
                text = deviceName,
                color = TextPrimary,
                fontSize = 24.sp
            )

            Spacer(modifier = Modifier.height(32.dp))

            Text(
                text = "DLNA + AirPlay 就绪",
                color = TextSecondary,
                fontSize = 18.sp
            )

            Spacer(modifier = Modifier.height(8.dp))

            if (localIp.isNotBlank()) {
                Text(
                    text = "DLNA: $localIp:$dlnaPort  |  AirPlay: $localIp:$airplayPort",
                    color = TextSecondary,
                    fontSize = 14.sp
                )
            }

            Spacer(modifier = Modifier.height(48.dp))

            Text(
                text = "从手机投屏视频到此设备",
                color = TextSecondary,
                fontSize = 16.sp
            )
        }
    }
}
