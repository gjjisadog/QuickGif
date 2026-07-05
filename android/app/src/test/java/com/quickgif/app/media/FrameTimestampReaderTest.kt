package com.quickgif.app.media

import com.quickgif.app.data.VideoInfo
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class FrameTimestampReaderTest {
    private val reader = FrameTimestampReader()

    @Test
    fun estimatesFrameCountFromReadableFrameCountFirst() {
        val videoInfo = videoInfo(durationMs = 5_000, frameRate = 30f, frameCount = 120)

        assertEquals(120L, reader.estimatedFrameCount(videoInfo))
    }

    @Test
    fun navigatesToPreviousAndNextFrameUsingTimestamps() {
        val videoInfo = videoInfo(durationMs = 1_000, frameRate = 10f, frameCount = 10)

        assertEquals(400L, reader.previousFrame(videoInfo, 500)?.timestampMs)
        assertEquals(600L, reader.nextFrame(videoInfo, 500)?.timestampMs)
        assertEquals(5L, reader.frameAtTimestamp(videoInfo, 540)?.index)
    }

    @Test
    fun clampsFrameNavigationToVideoBounds() {
        val videoInfo = videoInfo(durationMs = 1_000, frameRate = 10f, frameCount = 10)

        assertEquals(0L, reader.previousFrame(videoInfo, 0)?.timestampMs)
        assertEquals(900L, reader.nextFrame(videoInfo, 950)?.timestampMs)
    }

    @Test
    fun returnsNullWhenFrameRateIsMissing() {
        val videoInfo = videoInfo(durationMs = 1_000, frameRate = null, frameCount = null)

        assertNull(reader.nextFrame(videoInfo, 0))
    }

    private fun videoInfo(durationMs: Long, frameRate: Float?, frameCount: Long?): VideoInfo {
        return VideoInfo(
            displayName = "sample.mp4",
            durationMs = durationMs,
            width = 1920,
            height = 1080,
            frameRate = frameRate,
            codec = "video/avc",
            frameCount = frameCount
        )
    }
}
