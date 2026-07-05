package com.quickgif.app.ffmpeg

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ProgressParserTest {
    @Test
    fun parseProgressFromOutTimeUs() {
        val progress = ProgressParser.parseProgress("out_time_us=2500000", totalDurationMs = 5000)
        assertEquals(0.5f, progress ?: -1f, 0.001f)
    }

    @Test
    fun parseProgressFromTimestamp() {
        val progress = ProgressParser.parseProgress("out_time=00:00:03.000000", totalDurationMs = 6000)
        assertEquals(0.5f, progress ?: -1f, 0.001f)
    }

    @Test
    fun ignoreNonProgressLines() {
        assertNull(ProgressParser.parseProgress("frame=10", totalDurationMs = 6000))
    }
}
