package com.quickgif.app.ui

import android.content.Intent
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.quickgif.app.data.ExportState

@Composable
fun VideoPickerScreen(viewModel: QuickGifViewModel = viewModel()) {
    val state by viewModel.uiState.collectAsState()
    val context = LocalContext.current
    val snackbarHostState = remember { SnackbarHostState() }

    val photoPickerLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.PickVisualMedia(),
        onResult = { uri -> uri?.let(viewModel::loadVideo) }
    )
    val documentLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.OpenDocument(),
        onResult = { uri -> uri?.let(viewModel::loadVideo) }
    )

    LaunchedEffect(state.exportState) {
        when (val exportState = state.exportState) {
            is ExportState.Failure -> snackbarHostState.showSnackbar(exportState.message)
            is ExportState.Success -> snackbarHostState.showSnackbar("保存成功")
            else -> Unit
        }
    }

    MaterialTheme {
        Scaffold(snackbarHost = { SnackbarHost(snackbarHostState) }) { padding ->
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(padding)
                    .verticalScroll(rememberScrollState())
                    .padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(14.dp)
            ) {
                Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    Text(
                        modifier = Modifier.weight(1f),
                        text = "QuickGif",
                        style = MaterialTheme.typography.headlineMedium
                    )
                    Button(onClick = {
                        photoPickerLauncher.launch(PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.VideoOnly))
                    }) {
                        Text("相册")
                    }
                    Button(onClick = {
                        documentLauncher.launch(arrayOf("video/*"))
                    }) {
                        Text("文件")
                    }
                }

                VideoPreview(uri = state.selectedUri, seekToMs = state.currentTimestampMs)
                VideoInfoBlock(state)
                ThumbnailTimeline(
                    thumbnails = state.thumbnails,
                    currentTimestampMs = state.currentTimestampMs,
                    startTimestampMs = state.startTimestampMs,
                    endTimestampMs = state.endTimestampMs,
                    isGenerating = state.isGeneratingThumbnails,
                    onSelect = viewModel::selectTimelinePosition
                )
                FrameControlPanel(
                    currentTimestampMs = state.currentTimestampMs,
                    currentFrameIndex = state.currentFrameIndex,
                    startFrameIndex = state.startFrameIndex,
                    endFrameIndex = state.endFrameIndex,
                    enabled = state.videoInfo?.frameRate != null,
                    onPreviousFrame = viewModel::previousFrame,
                    onNextFrame = viewModel::nextFrame,
                    onSetStart = viewModel::setCurrentAsStartFrame,
                    onSetEnd = viewModel::setCurrentAsEndFrame
                )
                ExportSettingPanel(
                    state = state,
                    onStartChanged = viewModel::updateStartText,
                    onEndChanged = viewModel::updateEndText,
                    onWidthChanged = viewModel::updateWidth,
                    onFrameRateChanged = viewModel::updateFrameRate,
                    onQualityChanged = viewModel::updateQuality
                )

                Spacer(Modifier.height(4.dp))
                Button(
                    modifier = Modifier.fillMaxWidth(),
                    enabled = state.inputFile != null && state.exportState !is ExportState.Running,
                    onClick = viewModel::exportGif
                ) {
                    Text("导出 GIF")
                }

                if (state.exportState is ExportState.Success) {
                    val outputUri = (state.exportState as ExportState.Success).outputUri
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        TextButton(onClick = {
                            context.startActivity(Intent(Intent.ACTION_VIEW).apply {
                                setDataAndType(outputUri, "image/gif")
                                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                            })
                        }) {
                            Text("打开 GIF")
                        }
                        TextButton(onClick = {
                            context.startActivity(Intent.createChooser(Intent(Intent.ACTION_SEND).apply {
                                type = "image/gif"
                                putExtra(Intent.EXTRA_STREAM, outputUri)
                                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
                            }, "分享 GIF"))
                        }) {
                            Text("分享 GIF")
                        }
                    }
                }
            }
        }
        ExportProgressDialog(exportState = state.exportState, onCancel = viewModel::cancelExport)
    }
}

@Composable
private fun VideoInfoBlock(state: QuickGifUiState) {
    val info = state.videoInfo
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text("视频信息", style = MaterialTheme.typography.titleMedium)
        if (state.isLoadingVideo) {
            Text("正在读取视频...")
        } else if (info == null) {
            Text("尚未选择视频")
        } else {
            Text("文件：${info.displayName}")
            Text("时长：${"%.2f".format(info.durationMs / 1000.0)} 秒")
            Text("分辨率：${info.width ?: "-"} x ${info.height ?: "-"}")
            Text("帧率：${info.frameRate?.let { "%.2f".format(it) } ?: "-"}")
            Text("编码：${info.codec ?: "-"}")
            Text("总帧数：${info.frameCount ?: "-"}")
        }
    }
}
