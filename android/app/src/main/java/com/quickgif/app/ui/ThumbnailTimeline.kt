package com.quickgif.app.ui

import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.AssistChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.unit.dp
import com.quickgif.app.data.ThumbnailFrame

@Composable
fun ThumbnailTimeline(
    thumbnails: List<ThumbnailFrame>,
    currentTimestampMs: Long,
    startTimestampMs: Long,
    endTimestampMs: Long?,
    isGenerating: Boolean,
    onSelect: (Long) -> Unit,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier.fillMaxWidth()) {
        Row(verticalAlignment = Alignment.CenterVertically) {
            Text(
                modifier = Modifier.weight(1f),
                text = "时间轴",
                style = MaterialTheme.typography.titleMedium
            )
            AssistChip(onClick = {}, label = { Text(if (isGenerating) "生成中" else "${thumbnails.size} 张") })
        }
        Spacer(Modifier.height(8.dp))
        Box(
            modifier = Modifier
                .fillMaxWidth()
                .height(104.dp)
                .background(MaterialTheme.colorScheme.surfaceVariant, RoundedCornerShape(8.dp)),
            contentAlignment = Alignment.Center
        ) {
            if (thumbnails.isEmpty()) {
                Text(if (isGenerating) "正在生成缩略图..." else "选择视频后生成缩略图")
            } else {
                LazyRow(
                    modifier = Modifier.fillMaxWidth(),
                    contentPadding = PaddingValues(horizontal = 8.dp, vertical = 8.dp)
                ) {
                    items(thumbnails, key = { it.timestampMs }) { frame ->
                        val isCurrent = kotlin.math.abs(frame.timestampMs - currentTimestampMs) < 34L
                        val isStart = kotlin.math.abs(frame.timestampMs - startTimestampMs) < 34L
                        val isEnd = endTimestampMs?.let { kotlin.math.abs(frame.timestampMs - it) < 34L } == true
                        ThumbnailItem(
                            frame = frame,
                            isCurrent = isCurrent,
                            isStart = isStart,
                            isEnd = isEnd,
                            onClick = { onSelect(frame.timestampMs) }
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun ThumbnailItem(
    frame: ThumbnailFrame,
    isCurrent: Boolean,
    isStart: Boolean,
    isEnd: Boolean,
    onClick: () -> Unit
) {
    val borderColor = when {
        isCurrent -> MaterialTheme.colorScheme.primary
        isStart -> MaterialTheme.colorScheme.tertiary
        isEnd -> MaterialTheme.colorScheme.error
        else -> MaterialTheme.colorScheme.outlineVariant
    }
    Box(
        modifier = Modifier
            .padding(end = 6.dp)
            .width(72.dp)
            .height(88.dp)
            .clip(RoundedCornerShape(6.dp))
            .border(2.dp, borderColor, RoundedCornerShape(6.dp))
            .clickable(onClick = onClick)
    ) {
        Image(
            bitmap = frame.bitmap.asImageBitmap(),
            contentDescription = "缩略图 ${formatTimestamp(frame.timestampMs)}",
            modifier = Modifier.matchParentSize(),
            contentScale = ContentScale.Crop
        )
        Text(
            modifier = Modifier
                .align(Alignment.BottomCenter)
                .fillMaxWidth()
                .background(MaterialTheme.colorScheme.scrim.copy(alpha = 0.58f))
                .padding(vertical = 2.dp),
            text = markerText(isStart, isEnd) ?: formatTimestamp(frame.timestampMs),
            color = MaterialTheme.colorScheme.inverseOnSurface,
            style = MaterialTheme.typography.labelSmall
        )
    }
}

private fun markerText(isStart: Boolean, isEnd: Boolean): String? {
    return when {
        isStart && isEnd -> "起止"
        isStart -> "开始"
        isEnd -> "结束"
        else -> null
    }
}

private fun formatTimestamp(ms: Long): String {
    return "%.1fs".format(ms / 1000.0)
}
