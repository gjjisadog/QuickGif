package com.quickgif.app.ui

import android.net.Uri
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.media3.common.MediaItem
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.SeekParameters
import androidx.media3.ui.PlayerView

@Composable
fun VideoPreview(uri: Uri?, seekToMs: Long, modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val player = remember {
        ExoPlayer.Builder(context).build()
    }

    DisposableEffect(uri) {
        if (uri != null) {
            player.setSeekParameters(SeekParameters.EXACT)
            player.setMediaItem(MediaItem.fromUri(uri))
            player.prepare()
        }
        onDispose {
            player.pause()
        }
    }

    LaunchedEffect(seekToMs, uri) {
        if (uri != null) {
            player.seekTo(seekToMs.coerceAtLeast(0L))
        }
    }

    DisposableEffect(Unit) {
        onDispose {
            player.release()
        }
    }

    Box(
        modifier = modifier
            .fillMaxWidth()
            .height(240.dp)
            .background(MaterialTheme.colorScheme.surfaceVariant),
        contentAlignment = Alignment.Center
    ) {
        if (uri == null) {
            Text("选择视频后在这里预览")
        } else {
            AndroidView(
                modifier = Modifier.fillMaxSize(),
                factory = { PlayerView(it).apply { this.player = player } },
                update = { it.player = player }
            )
        }
    }
}
