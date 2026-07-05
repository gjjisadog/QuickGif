package com.quickgif.app.storage

import android.content.Context
import android.net.Uri
import com.quickgif.app.error.AppError
import java.io.File

class TempFileManager(private val context: Context) {
    fun copyUriToCache(uri: Uri, displayName: String): File {
        return try {
            val safeName = displayName.replace(Regex("[^A-Za-z0-9._-]"), "_").ifBlank { "input_video" }
            val extension = safeName.substringAfterLast('.', "mp4")
            val inputFile = File.createTempFile("quickgif_input_", ".$extension", context.cacheDir)
            context.contentResolver.openInputStream(uri).use { input ->
                requireNotNull(input) { "无法打开视频输入流" }
                inputFile.outputStream().use { output -> input.copyTo(output) }
            }
            inputFile
        } catch (throwable: Throwable) {
            throw AppError.TempFileFailed(throwable)
        }
    }

    fun createPaletteFile(): File = File.createTempFile("quickgif_palette_", ".png", context.cacheDir)

    fun createGifFile(baseName: String): File {
        val normalized = baseName.substringBeforeLast('.').ifBlank { "quickgif" }
            .replace(Regex("[^A-Za-z0-9._-]"), "_")
        return File(context.cacheDir, "${normalized}_${System.currentTimeMillis()}.gif")
    }

    fun cleanup(files: Iterable<File>) {
        files.forEach { file ->
            if (file.exists()) file.delete()
        }
    }
}
