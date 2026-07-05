package com.quickgif.app.ffmpeg

import android.content.Context
import com.quickgif.app.error.AppError
import java.io.File

class NativeLibraryLoader(private val context: Context) {
    fun ffmpegExecutable(): File {
        val file = File(context.applicationInfo.nativeLibraryDir, "libffmpeg.so")
        if (!file.exists()) {
            throw AppError.FfmpegMissing(file.absolutePath)
        }
        return file
    }
}
