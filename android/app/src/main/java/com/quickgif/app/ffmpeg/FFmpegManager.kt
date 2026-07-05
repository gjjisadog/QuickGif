package com.quickgif.app.ffmpeg

import android.content.Context
import com.quickgif.app.error.AppError
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File

class FFmpegManager(context: Context) {
    private val loader = NativeLibraryLoader(context.applicationContext)
    @Volatile private var runningProcess: Process? = null

    suspend fun run(
        args: List<String>,
        totalDurationMs: Long,
        progressOffset: Float,
        progressScale: Float,
        onProgress: (Float) -> Unit
    ) = withContext(Dispatchers.IO) {
        val executable = loader.ffmpegExecutable()
        val process = ProcessBuilder(listOf(executable.absolutePath) + args)
            .redirectErrorStream(true)
            .start()
        runningProcess = process

        try {
            process.inputStream.bufferedReader().useLines { lines ->
                lines.forEach { line ->
                    ProgressParser.parseProgress(line, totalDurationMs)?.let { partProgress ->
                        onProgress((progressOffset + partProgress * progressScale).coerceIn(0f, 1f))
                    }
                }
            }

            val exitCode = process.waitFor()
            if (exitCode != 0) {
                throw AppError.FfmpegFailed("FFmpeg 退出码 $exitCode")
            }
        } finally {
            runningProcess = null
        }
    }

    fun cancel() {
        runningProcess?.destroy()
        runningProcess = null
    }
}
