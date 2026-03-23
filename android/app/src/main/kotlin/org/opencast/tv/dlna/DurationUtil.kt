package org.opencast.tv.dlna

object DurationUtil {
    fun formatDuration(secs: Double): String {
        val total = secs.toLong()
        val h = total / 3600
        val m = (total % 3600) / 60
        val s = total % 60
        return "%02d:%02d:%02d".format(h, m, s)
    }

    fun parseDuration(s: String): Double {
        val parts = s.split(":")
        if (parts.size == 3) {
            val h = parts[0].toDoubleOrNull() ?: 0.0
            val m = parts[1].toDoubleOrNull() ?: 0.0
            val sec = parts[2].toDoubleOrNull() ?: 0.0
            return h * 3600 + m * 60 + sec
        }
        return 0.0
    }
}
