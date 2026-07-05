package com.quickgif.app.media

import android.graphics.Bitmap
import android.media.MediaMetadataRetriever
import com.quickgif.app.data.ThumbnailFrame
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import kotlin.math.roundToLong

class ThumbnailExtractor {
    suspend fun extract(inputFile: File, durationMs: Long, maxCount: Int = MAX_THUMBNAILS): List<ThumbnailFrame> {
        val timestamps = planTimestamps(durationMs, maxCount)
        if (timestamps.isEmpty()) return emptyList()

        return withContext(Dispatchers.IO) {
            MediaMetadataRetriever().use { retriever ->
                retriever.setDataSource(inputFile.absolutePath)
                timestamps.mapNotNull { timestampMs ->
                    retriever.getFrameAtTime(timestampMs * 1000L, MediaMetadataRetriever.OPTION_CLOSEST)
                        ?.let { bitmap ->
                            ThumbnailFrame(
                                timestampMs = timestampMs,
                                bitmap = bitmap.scaledToTimeline()
                            )
                        }
                }
            }
        }
    }

    private fun Bitmap.scaledToTimeline(): Bitmap {
        val targetHeight = 96
        if (height <= targetHeight) return this
        val targetWidth = (width * (targetHeight.toFloat() / height.toFloat())).roundToLong().toInt().coerceAtLeast(1)
        val scaled = Bitmap.createScaledBitmap(this, targetWidth, targetHeight, true)
        if (scaled !== this) recycle()
        return scaled
    }

    companion object {
        const val MAX_THUMBNAILS = 80

        fun planTimestamps(durationMs: Long, maxCount: Int = MAX_THUMBNAILS): List<Long> {
            if (durationMs <= 0L || maxCount <= 0) return emptyList()
            if (maxCount == 1) return listOf(0L)

            val step = durationMs.toDouble() / (maxCount - 1).toDouble()
            return List(maxCount) { index ->
                if (index == maxCount - 1) {
                    durationMs
                } else {
                    (index * step).roundToLong().coerceIn(0L, durationMs)
                }
            }.distinct()
        }
    }
}
