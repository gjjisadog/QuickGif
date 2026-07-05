package com.quickgif.app.media

import androidx.media3.exoplayer.ExoPlayer

class VideoPlayerController(private val player: ExoPlayer) {
    fun seekTo(ms: Long) {
        player.seekTo(ms.coerceAtLeast(0L))
    }

    fun playPause() {
        if (player.isPlaying) {
            player.pause()
        } else {
            player.play()
        }
    }
}
