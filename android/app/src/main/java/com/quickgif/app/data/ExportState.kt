package com.quickgif.app.data

import android.net.Uri

sealed interface ExportState {
    data object Idle : ExportState
    data class Running(val progress: Float, val message: String) : ExportState
    data class Success(val outputUri: Uri) : ExportState
    data class Failure(val message: String) : ExportState
}
