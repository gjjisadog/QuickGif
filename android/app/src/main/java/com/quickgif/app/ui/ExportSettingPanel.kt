package com.quickgif.app.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.FilterChip
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.quickgif.app.data.GifFrameRateOption
import com.quickgif.app.data.GifQualityOption
import com.quickgif.app.data.GifWidthOption

@Composable
fun ExportSettingPanel(
    state: QuickGifUiState,
    onStartChanged: (String) -> Unit,
    onEndChanged: (String) -> Unit,
    onWidthChanged: (GifWidthOption) -> Unit,
    onFrameRateChanged: (GifFrameRateOption) -> Unit,
    onQualityChanged: (GifQualityOption) -> Unit,
    modifier: Modifier = Modifier
) {
    Column(modifier = modifier, verticalArrangement = Arrangement.spacedBy(10.dp)) {
        Text("导出片段", style = MaterialTheme.typography.titleMedium)
        Row(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
            OutlinedTextField(
                modifier = Modifier.weight(1f),
                value = state.startText,
                onValueChange = onStartChanged,
                label = { Text("开始秒") },
                singleLine = true
            )
            OutlinedTextField(
                modifier = Modifier.weight(1f),
                value = state.endText,
                onValueChange = onEndChanged,
                label = { Text("结束秒") },
                singleLine = true
            )
        }

        SettingChips(
            title = "宽度",
            values = GifWidthOption.entries,
            selected = state.width,
            label = { it.label },
            onSelected = onWidthChanged
        )
        SettingChips(
            title = "帧率",
            values = GifFrameRateOption.entries,
            selected = state.frameRate,
            label = { it.label },
            onSelected = onFrameRateChanged
        )
        SettingChips(
            title = "质量",
            values = GifQualityOption.entries,
            selected = state.quality,
            label = { it.label },
            onSelected = onQualityChanged
        )
    }
}

@Composable
private fun <T> SettingChips(
    title: String,
    values: List<T>,
    selected: T,
    label: (T) -> String,
    onSelected: (T) -> Unit
) {
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(title, style = MaterialTheme.typography.labelLarge)
        Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.spacedBy(8.dp)
        ) {
            values.forEach { value ->
                FilterChip(
                    selected = value == selected,
                    onClick = { onSelected(value) },
                    label = { Text(label(value)) }
                )
            }
        }
    }
}
