package com.quickgif.app.ffmpeg

import java.util.Locale

object ProgressParser {
    fun parseProgress(line: String, totalDurationMs: Long): Float? {
        if (totalDurationMs <= 0L) return null
        val parts = line.split("=", limit = 2)
        if (parts.size != 2) return null

        val elapsedMs = when (parts[0]) {
            "out_time_ms", "out_time_us" -> parts[1].toLongOrNull()?.let { it / 1000L }
            "out_time" -> parseTimestamp(parts[1])
            else -> null
        } ?: return null

        return (elapsedMs.toFloat() / totalDurationMs.toFloat()).coerceIn(0f, 1f)
    }

    private fun parseTimestamp(value: String): Long? {
        val segments = value.split(":")
        if (segments.size != 3) return null

        val hours = segments[0].toLongOrNull() ?: return null
        val minutes = segments[1].toLongOrNull() ?: return null
        val seconds = segments[2].toDoubleOrNull() ?: return null

        return ((hours * 3600 + minutes * 60) * 1000 + seconds * 1000).toLong()
    }

    fun formatPercent(progress: Float): String {
        return String.format(Locale.US, "%.0f%%", progress.coerceIn(0f, 1f) * 100f)
    }
}
