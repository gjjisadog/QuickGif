package com.quickgif.app.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import com.quickgif.app.data.ExportState

@Composable
fun ExportProgressDialog(exportState: ExportState, onCancel: () -> Unit) {
    if (exportState is ExportState.Running) {
        AlertDialog(
            onDismissRequest = {},
            confirmButton = {
                TextButton(onClick = onCancel) {
                    Text("取消")
                }
            },
            title = { Text("正在导出") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                    Text(exportState.message)
                    LinearProgressIndicator(
                        progress = { exportState.progress },
                        modifier = Modifier.fillMaxWidth()
                    )
                }
            }
        )
    }
}
