package com.quickgif.app.media

import android.content.Context
import android.media.MediaMetadataRetriever
import android.net.Uri
import android.provider.OpenableColumns
import com.quickgif.app.data.VideoInfo
import com.quickgif.app.error.AppError

class VideoInfoReader(private val context: Context) {
    fun read(uri: Uri): VideoInfo {
        return try {
            MediaMetadataRetriever().use { retriever ->
                retriever.setDataSource(context, uri)
                val width = retriever.extractInt(MediaMetadataRetriever.METADATA_KEY_VIDEO_WIDTH)
                val height = retriever.extractInt(MediaMetadataRetriever.METADATA_KEY_VIDEO_HEIGHT)
                val durationMs = retriever.extractLong(MediaMetadataRetriever.METADATA_KEY_DURATION) ?: 0L
                val frameRate = retriever.extractString(MediaMetadataRetriever.METADATA_KEY_CAPTURE_FRAMERATE)?.toFloatOrNull()
                val codec = retriever.extractString(MediaMetadataRetriever.METADATA_KEY_MIMETYPE)
                val frameCount = retriever.extractLong(MediaMetadataRetriever.METADATA_KEY_VIDEO_FRAME_COUNT)

                VideoInfo(
                    displayName = displayName(uri),
                    durationMs = durationMs,
                    width = width,
                    height = height,
                    frameRate = frameRate,
                    codec = codec,
                    frameCount = frameCount
                )
            }
        } catch (throwable: Throwable) {
            throw AppError.VideoReadFailed(throwable)
        }
    }

    private fun MediaMetadataRetriever.extractInt(keyCode: Int): Int? {
        return extractMetadata(keyCode)?.toIntOrNull()?.takeIf { it > 0 }
    }

    private fun MediaMetadataRetriever.extractLong(keyCode: Int): Long? {
        return extractMetadata(keyCode)?.toLongOrNull()?.takeIf { it >= 0L }
    }

    private fun MediaMetadataRetriever.extractString(keyCode: Int): String? {
        return extractMetadata(keyCode)?.takeIf { it.isNotBlank() }
    }

    private fun displayName(uri: Uri): String {
        context.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
            val index = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            if (index >= 0 && cursor.moveToFirst()) {
                return cursor.getString(index)
            }
        }
        return uri.lastPathSegment ?: "video"
    }
}
