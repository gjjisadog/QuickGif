package com.quickgif.app.ui

import android.app.Application
import android.net.Uri
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.quickgif.app.data.ExportState
import com.quickgif.app.data.GifExportConfig
import com.quickgif.app.data.GifFrameRateOption
import com.quickgif.app.data.GifQualityOption
import com.quickgif.app.data.GifWidthOption
import com.quickgif.app.data.ThumbnailFrame
import com.quickgif.app.data.VideoInfo
import com.quickgif.app.error.AppError
import com.quickgif.app.error.ErrorMapper
import com.quickgif.app.ffmpeg.GifExporter
import com.quickgif.app.media.FrameTimestampReader
import com.quickgif.app.media.ThumbnailExtractor
import com.quickgif.app.media.VideoInfoReader
import com.quickgif.app.storage.TempFileManager
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import java.io.File

data class QuickGifUiState(
    val selectedUri: Uri? = null,
    val videoInfo: VideoInfo? = null,
    val inputFile: File? = null,
    val startText: String = "0.0",
    val endText: String = "",
    val width: GifWidthOption = GifWidthOption.W480,
    val frameRate: GifFrameRateOption = GifFrameRateOption.Fps15,
    val quality: GifQualityOption = GifQualityOption.Standard,
    val exportState: ExportState = ExportState.Idle,
    val isLoadingVideo: Boolean = false,
    val isGeneratingThumbnails: Boolean = false,
    val thumbnails: List<ThumbnailFrame> = emptyList(),
    val currentTimestampMs: Long = 0L,
    val currentFrameIndex: Long? = null,
    val startFrameIndex: Long? = null,
    val startTimestampMs: Long = 0L,
    val endFrameIndex: Long? = null,
    val endTimestampMs: Long? = null
)

class QuickGifViewModel(application: Application) : AndroidViewModel(application) {
    private val context = application.applicationContext
    private val videoInfoReader = VideoInfoReader(context)
    private val frameTimestampReader = FrameTimestampReader()
    private val thumbnailExtractor = ThumbnailExtractor()
    private val tempFileManager = TempFileManager(context)
    private val gifExporter = GifExporter(context)
    private var exportJob: Job? = null
    private var thumbnailJob: Job? = null
    private var tempFiles: MutableList<File> = mutableListOf()

    private val _uiState = MutableStateFlow(QuickGifUiState())
    val uiState: StateFlow<QuickGifUiState> = _uiState.asStateFlow()

    fun loadVideo(uri: Uri) {
        viewModelScope.launch {
            thumbnailJob?.cancel()
            clearThumbnails()
            _uiState.update {
                QuickGifUiState(
                    selectedUri = uri,
                    isLoadingVideo = true,
                    exportState = ExportState.Idle
                )
            }
            runCatching {
                val info = videoInfoReader.read(uri)
                val inputFile = tempFileManager.copyUriToCache(uri, info.displayName)
                tempFiles += inputFile
                info to inputFile
            }.onSuccess { (info, inputFile) ->
                _uiState.update {
                    it.copy(
                        selectedUri = uri,
                        videoInfo = info,
                        inputFile = inputFile,
                        startText = "0.0",
                        endText = "%.1f".format(info.durationMs / 1000.0),
                        endTimestampMs = info.durationMs,
                        currentFrameIndex = frameTimestampReader.frameAtTimestamp(info, 0L)?.index,
                        endFrameIndex = frameTimestampReader.frameAtTimestamp(info, info.durationMs)?.index,
                        isLoadingVideo = false
                    )
                }
                generateThumbnails(inputFile, info.durationMs)
            }.onFailure { throwable ->
                _uiState.update {
                    it.copy(
                        isLoadingVideo = false,
                        exportState = ExportState.Failure(ErrorMapper.toUserMessage(throwable))
                    )
                }
            }
        }
    }

    fun selectTimelinePosition(timestampMs: Long) {
        val info = _uiState.value.videoInfo ?: return
        val frame = frameTimestampReader.frameAtTimestamp(info, timestampMs)
        _uiState.update {
            it.copy(
                currentTimestampMs = frame?.timestampMs ?: timestampMs.coerceIn(0L, info.durationMs),
                currentFrameIndex = frame?.index
            )
        }
    }

    fun previousFrame() {
        val state = _uiState.value
        val info = state.videoInfo ?: return
        frameTimestampReader.previousFrame(info, state.currentTimestampMs)?.let { frame ->
            _uiState.update {
                it.copy(currentTimestampMs = frame.timestampMs, currentFrameIndex = frame.index)
            }
        }
    }

    fun nextFrame() {
        val state = _uiState.value
        val info = state.videoInfo ?: return
        frameTimestampReader.nextFrame(info, state.currentTimestampMs)?.let { frame ->
            _uiState.update {
                it.copy(currentTimestampMs = frame.timestampMs, currentFrameIndex = frame.index)
            }
        }
    }

    fun setCurrentAsStartFrame() {
        val state = _uiState.value
        val startMs = state.currentTimestampMs
        val boundedEndMs = state.endTimestampMs?.coerceAtLeast(startMs + 1L)
        _uiState.update {
            it.copy(
                startTimestampMs = startMs,
                startFrameIndex = state.currentFrameIndex,
                endTimestampMs = boundedEndMs,
                startText = formatSeconds(startMs),
                endText = boundedEndMs?.let(::formatSeconds) ?: it.endText
            )
        }
    }

    fun setCurrentAsEndFrame() {
        val state = _uiState.value
        val endMs = state.currentTimestampMs.coerceAtLeast(state.startTimestampMs + 1L)
        _uiState.update {
            it.copy(
                endTimestampMs = endMs,
                endFrameIndex = state.currentFrameIndex,
                endText = formatSeconds(endMs)
            )
        }
    }

    fun updateStartText(value: String) {
        val startMs = parseSecondsToMs(value)
        _uiState.update {
            it.copy(
                startText = value,
                startTimestampMs = startMs ?: it.startTimestampMs,
                startFrameIndex = startMs?.let { ms -> it.videoInfo?.let { info -> frameTimestampReader.frameAtTimestamp(info, ms)?.index } } ?: it.startFrameIndex
            )
        }
    }

    fun updateEndText(value: String) {
        val endMs = parseSecondsToMs(value)
        _uiState.update {
            it.copy(
                endText = value,
                endTimestampMs = endMs ?: it.endTimestampMs,
                endFrameIndex = endMs?.let { ms -> it.videoInfo?.let { info -> frameTimestampReader.frameAtTimestamp(info, ms)?.index } } ?: it.endFrameIndex
            )
        }
    }

    fun updateWidth(value: GifWidthOption) {
        _uiState.update { it.copy(width = value) }
    }

    fun updateFrameRate(value: GifFrameRateOption) {
        _uiState.update { it.copy(frameRate = value) }
    }

    fun updateQuality(value: GifQualityOption) {
        _uiState.update { it.copy(quality = value) }
    }

    fun exportGif() {
        val state = _uiState.value
        val input = state.inputFile ?: return
        val startMs = parseSecondsToMs(state.startText)
        val endMs = parseSecondsToMs(state.endText)
        if (startMs == null || endMs == null || endMs <= startMs) {
            _uiState.update { it.copy(exportState = ExportState.Failure(AppError.InvalidRange().message ?: "时间范围无效")) }
            return
        }

        val paletteFile = tempFileManager.createPaletteFile()
        val outputFile = tempFileManager.createGifFile(state.videoInfo?.displayName ?: "quickgif")
        tempFiles += paletteFile
        tempFiles += outputFile

        val config = GifExportConfig(
            inputFile = input,
            outputFile = outputFile,
            paletteFile = paletteFile,
            startMs = startMs,
            endMs = endMs,
            width = state.width,
            frameRate = state.frameRate,
            quality = state.quality
        )

        exportJob = viewModelScope.launch {
            _uiState.update { it.copy(exportState = ExportState.Running(0f, "正在导出 GIF")) }
            runCatching {
                gifExporter.export(config) { progress ->
                    _uiState.update {
                        it.copy(exportState = ExportState.Running(progress, "正在导出 ${com.quickgif.app.ffmpeg.ProgressParser.formatPercent(progress)}"))
                    }
                }
            }.onSuccess { uri ->
                _uiState.update { it.copy(exportState = ExportState.Success(uri)) }
                tempFileManager.cleanup(listOf(paletteFile, outputFile))
                tempFiles.removeAll(listOf(paletteFile, outputFile))
            }.onFailure { throwable ->
                _uiState.update { it.copy(exportState = ExportState.Failure(ErrorMapper.toUserMessage(throwable))) }
            }
        }
    }

    fun cancelExport() {
        gifExporter.cancel()
        exportJob?.cancel()
        _uiState.update { it.copy(exportState = ExportState.Idle) }
    }

    override fun onCleared() {
        thumbnailJob?.cancel()
        clearThumbnails()
        tempFileManager.cleanup(tempFiles)
        super.onCleared()
    }

    private fun generateThumbnails(inputFile: File, durationMs: Long) {
        thumbnailJob = viewModelScope.launch {
            _uiState.update { it.copy(isGeneratingThumbnails = true, thumbnails = emptyList()) }
            runCatching {
                thumbnailExtractor.extract(inputFile, durationMs)
            }.onSuccess { thumbnails ->
                _uiState.update { it.copy(thumbnails = thumbnails, isGeneratingThumbnails = false) }
            }.onFailure {
                _uiState.update { it.copy(isGeneratingThumbnails = false) }
            }
        }
    }

    private fun clearThumbnails() {
        _uiState.value.thumbnails.forEach { frame ->
            if (!frame.bitmap.isRecycled) frame.bitmap.recycle()
        }
    }

    private fun parseSecondsToMs(value: String): Long? {
        return value.trim().toDoubleOrNull()?.let { (it * 1000).toLong() }
    }

    private fun formatSeconds(ms: Long): String {
        return "%.3f".format(ms / 1000.0)
    }
}
