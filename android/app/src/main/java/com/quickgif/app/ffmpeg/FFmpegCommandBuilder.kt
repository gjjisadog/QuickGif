package com.quickgif.app.ffmpeg

import com.quickgif.app.data.GifExportConfig
import com.quickgif.app.data.GifFrameRateOption
import com.quickgif.app.data.GifWidthOption
import java.util.Locale

data class GifCommandSet(
    val paletteCommand: List<String>,
    val gifCommand: List<String>
)

class FFmpegCommandBuilder {
    fun build(config: GifExportConfig): GifCommandSet {
        val startSeconds = formatSeconds(config.startMs)
        val durationSeconds = formatSeconds(config.durationMs)
        val baseVideoFilter = buildVideoFilter(config.width, config.frameRate)

        val paletteCommand = listOf(
            "-y",
            "-ss", startSeconds,
            "-t", durationSeconds,
            "-i", config.inputFile.absolutePath,
            "-vf", "$baseVideoFilter,palettegen=max_colors=${config.quality.maxColors}",
            "-progress", "pipe:1",
            "-nostats",
            config.paletteFile.absolutePath
        )

        val gifCommand = listOf(
            "-y",
            "-ss", startSeconds,
            "-t", durationSeconds,
            "-i", config.inputFile.absolutePath,
            "-i", config.paletteFile.absolutePath,
            "-lavfi", "$baseVideoFilter[x];[x][1:v]paletteuse=dither=${config.quality.dither}",
            "-loop", "0",
            "-progress", "pipe:1",
            "-nostats",
            config.outputFile.absolutePath
        )

        return GifCommandSet(paletteCommand, gifCommand)
    }

    private fun buildVideoFilter(width: GifWidthOption, frameRate: GifFrameRateOption): String {
        val filters = mutableListOf<String>()
        if (frameRate.isNotOriginalFps) {
            filters += "fps=${frameRate.fps}"
        }
        filters += when (width) {
            GifWidthOption.Original -> "scale=iw:-1:flags=lanczos"
            else -> "scale=${width.pixels}:-1:flags=lanczos"
        }
        return filters.joinToString(",")
    }

    private val GifFrameRateOption.isNotOriginalFps: Boolean
        get() = this != GifFrameRateOption.Original && fps != null

    private fun formatSeconds(ms: Long): String {
        return String.format(Locale.US, "%.3f", ms / 1000.0)
    }
}
