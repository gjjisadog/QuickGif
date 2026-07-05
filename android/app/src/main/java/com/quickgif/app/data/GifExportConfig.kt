package com.quickgif.app.data

import java.io.File

enum class GifWidthOption(val label: String, val pixels: Int?) {
    Original("原始", null),
    W720("720", 720),
    W480("480", 480),
    W320("320", 320)
}

enum class GifFrameRateOption(val label: String, val fps: Int?) {
    Original("原始", null),
    Fps24("24 fps", 24),
    Fps15("15 fps", 15),
    Fps10("10 fps", 10)
}

enum class GifQualityOption(
    val label: String,
    val maxColors: Int,
    val dither: String
) {
    Small("小体积", 128, "bayer"),
    Standard("标准", 192, "sierra2_4a"),
    High("高清", 256, "sierra2_4a")
}

data class GifExportConfig(
    val inputFile: File,
    val outputFile: File,
    val paletteFile: File,
    val startMs: Long,
    val endMs: Long,
    val width: GifWidthOption,
    val frameRate: GifFrameRateOption,
    val quality: GifQualityOption
) {
    val durationMs: Long = (endMs - startMs).coerceAtLeast(1L)
}
