package com.quickgif.app.media

import com.quickgif.app.data.FrameInfo
import com.quickgif.app.data.VideoInfo

class FrameTimestampReader {
    fun approximateFrameAt(videoInfo: VideoInfo, timestampMs: Long): FrameInfo? {
        return frameAtTimestamp(videoInfo, timestampMs)
    }

    fun estimatedFrameCount(videoInfo: VideoInfo): Long? {
        videoInfo.frameCount?.takeIf { it > 0L }?.let { return it }
        val fps = videoInfo.frameRate?.takeIf { it > 0f } ?: return null
        return ((videoInfo.durationMs / 1000.0) * fps).toLong().coerceAtLeast(1L)
    }

    fun frameAtTimestamp(videoInfo: VideoInfo, timestampMs: Long): FrameInfo? {
        val fps = videoInfo.frameRate?.takeIf { it > 0f } ?: return null
        val frameCount = estimatedFrameCount(videoInfo) ?: return null
        val frameDurationMs = 1000.0 / fps
        val clampedTimestamp = timestampMs.coerceIn(0L, videoInfo.durationMs)
        val index = (clampedTimestamp / frameDurationMs).toLong().coerceIn(0L, frameCount - 1L)
        return frameAtIndex(videoInfo, index)
    }

    fun previousFrame(videoInfo: VideoInfo, timestampMs: Long): FrameInfo? {
        val current = frameAtTimestamp(videoInfo, timestampMs) ?: return null
        return frameAtIndex(videoInfo, current.index - 1L)
    }

    fun nextFrame(videoInfo: VideoInfo, timestampMs: Long): FrameInfo? {
        val current = frameAtTimestamp(videoInfo, timestampMs) ?: return null
        return frameAtIndex(videoInfo, current.index + 1L)
    }

    fun frameAtIndex(videoInfo: VideoInfo, frameIndex: Long): FrameInfo? {
        val fps = videoInfo.frameRate?.takeIf { it > 0f } ?: return null
        val frameCount = estimatedFrameCount(videoInfo) ?: return null
        val index = frameIndex.coerceIn(0L, frameCount - 1L)
        val timestamp = ((index * 1000.0) / fps).toLong().coerceIn(0L, videoInfo.durationMs)
        return FrameInfo(index, timestamp)
    }
}
