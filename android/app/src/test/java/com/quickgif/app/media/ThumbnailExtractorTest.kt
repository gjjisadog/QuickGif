package com.quickgif.app.media

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ThumbnailExtractorTest {
    @Test
    fun plansAtMostEightyThumbnailsAcrossVideoDuration() {
        val timestamps = ThumbnailExtractor.planTimestamps(durationMs = 240_000, maxCount = 80)

        assertEquals(80, timestamps.size)
        assertEquals(0, timestamps.first())
        assertTrue(timestamps.last() <= 240_000)
        assertTrue(timestamps.zipWithNext().all { (left, right) -> right > left })
    }

    @Test
    fun plansShortVideoWithBoundedEvenSamples() {
        val timestamps = ThumbnailExtractor.planTimestamps(durationMs = 5_000, maxCount = 8)

        assertEquals(listOf(0L, 714L, 1429L, 2143L, 2857L, 3571L, 4286L, 5000L), timestamps)
    }

    @Test
    fun returnsEmptyPlanForInvalidDuration() {
        assertEquals(emptyList<Long>(), ThumbnailExtractor.planTimestamps(durationMs = 0, maxCount = 80))
    }
}
