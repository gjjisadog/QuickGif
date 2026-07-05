package com.quickgif.app.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

@Composable
fun FrameControlPanel(
    currentTimestampMs: Long,
    currentFrameIndex: Long?,
    startFrameIndex: Long?,
    endFrameIndex: Long?,
    enabled: Boolean,
    onPreviousFrame: () -> Unit,
    onNextFrame: () -> Unit,
    onSetStart: () -> Unit,
    onSetEnd: () -> Unit,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier.fillMaxWidth(), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Text("逐帧控制", style = MaterialTheme.typography.titleMedium)
        Text(
            text = "当前：${currentFrameIndex?.let { "#$it" } ?: "-"} / ${formatTimestamp(currentTimestampMs)}  开始：${startFrameIndex?.let { "#$it" } ?: "-"}  结束：${endFrameIndex?.let { "#$it" } ?: "-"}",
            style = MaterialTheme.typography.bodyMedium
        )
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedButton(modifier = Modifier.weight(1f), enabled = enabled, onClick = onPreviousFrame) {
                Text("上一帧")
            }
            OutlinedButton(modifier = Modifier.weight(1f), enabled = enabled, onClick = onNextFrame) {
                Text("下一帧")
            }
        }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(modifier = Modifier.weight(1f), enabled = enabled, onClick = onSetStart) {
                Text("设为开始帧")
            }
            Button(modifier = Modifier.weight(1f), enabled = enabled, onClick = onSetEnd) {
                Text("设为结束帧")
            }
        }
    }
}

private fun formatTimestamp(ms: Long): String {
    return "%.3fs".format(ms / 1000.0)
}
