package com.quickgif.app.error

object UserFriendlyErrorMessage {
    fun exportFailed(detail: String): String = "导出失败：$detail"
}
