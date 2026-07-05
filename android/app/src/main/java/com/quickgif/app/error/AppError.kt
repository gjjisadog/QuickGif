package com.quickgif.app.error

sealed class AppError(message: String, cause: Throwable? = null) : Exception(message, cause) {
    class VideoReadFailed(cause: Throwable) : AppError("无法读取视频信息", cause)
    class TempFileFailed(cause: Throwable) : AppError("无法创建临时文件", cause)
    class FfmpegMissing(path: String) : AppError("未找到内置 FFmpeg：$path")
    class FfmpegFailed(message: String) : AppError(message)
    class SaveFailed(cause: Throwable) : AppError("GIF 保存失败", cause)
    class InvalidRange : AppError("结束时间必须大于开始时间")
}
