package com.quickgif.app.error

object ErrorMapper {
    fun toUserMessage(throwable: Throwable): String {
        return when (throwable) {
            is AppError -> throwable.message ?: "操作失败"
            else -> throwable.message ?: "发生未知错误"
        }
    }
}
