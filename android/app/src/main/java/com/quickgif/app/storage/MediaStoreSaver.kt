package com.quickgif.app.storage

import android.content.ContentValues
import android.content.Context
import android.net.Uri
import android.os.Build
import android.provider.MediaStore
import com.quickgif.app.error.AppError
import java.io.File

class MediaStoreSaver(private val context: Context) {
    fun saveGif(source: File, displayName: String): Uri {
        return try {
            val values = ContentValues().apply {
                put(MediaStore.MediaColumns.DISPLAY_NAME, displayName)
                put(MediaStore.MediaColumns.MIME_TYPE, "image/gif")
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    put(MediaStore.MediaColumns.RELATIVE_PATH, OutputPathManager.RELATIVE_MOVIES_DIR)
                    put(MediaStore.MediaColumns.IS_PENDING, 1)
                }
            }

            val collection = MediaStore.Files.getContentUri(MediaStore.VOLUME_EXTERNAL_PRIMARY)
            val uri = requireNotNull(context.contentResolver.insert(collection, values)) {
                "MediaStore insert returned null"
            }

            context.contentResolver.openOutputStream(uri).use { output ->
                requireNotNull(output) { "无法打开保存输出流" }
                source.inputStream().use { input -> input.copyTo(output) }
            }

            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                val publishedValues = ContentValues().apply {
                    put(MediaStore.MediaColumns.IS_PENDING, 0)
                }
                context.contentResolver.update(uri, publishedValues, null, null)
            }

            uri
        } catch (throwable: Throwable) {
            throw AppError.SaveFailed(throwable)
        }
    }
}
