package com.quickgif.app.ffmpeg

import android.content.Context
import com.quickgif.app.data.GifExportConfig
import com.quickgif.app.storage.MediaStoreSaver
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class GifExporter(context: Context) {
    private val appContext = context.applicationContext
    private val commandBuilder = FFmpegCommandBuilder()
    private val ffmpegManager = FFmpegManager(appContext)
    private val saver = MediaStoreSaver(appContext)

    suspend fun export(config: GifExportConfig, onProgress: (Float) -> Unit) = withContext(Dispatchers.IO) {
        val commands = commandBuilder.build(config)
        onProgress(0f)
        ffmpegManager.run(
            args = commands.paletteCommand,
            totalDurationMs = config.durationMs,
            progressOffset = 0f,
            progressScale = 0.35f,
            onProgress = onProgress
        )
        ffmpegManager.run(
            args = commands.gifCommand,
            totalDurationMs = config.durationMs,
            progressOffset = 0.35f,
            progressScale = 0.6f,
            onProgress = onProgress
        )
        onProgress(0.97f)
        val saved = saver.saveGif(config.outputFile, config.outputFile.name)
        onProgress(1f)
        saved
    }

    fun cancel() {
        ffmpegManager.cancel()
    }
}
