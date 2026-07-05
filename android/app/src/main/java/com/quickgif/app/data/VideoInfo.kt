package com.quickgif.app.data

data class VideoInfo(
    val displayName: String,
    val durationMs: Long,
    val width: Int?,
    val height: Int?,
    val frameRate: Float?,
    val codec: String?,
    val frameCount: Long?
)
