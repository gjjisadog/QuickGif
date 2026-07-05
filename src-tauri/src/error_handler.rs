use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum QuickGifError {
    #[error("文件不存在：{0}")]
    FileNotFound(PathBuf),
    #[error("暂不支持该视频格式：{0}")]
    UnsupportedFormat(String),
    #[error("未找到 {tool}。请将 {tool} 放入 src-tauri/binaries，或设置 QUICKGIF_FFMPEG_DIR。")]
    ToolNotFound { tool: String },
    #[error("{tool} 调用失败：{message}")]
    ToolFailed { tool: String, message: String },
    #[error("无法解析视频信息：{0}")]
    ProbeParse(String),
    #[error("起止时间无效：结束时间必须大于开始时间")]
    InvalidRange,
    #[error("{0}")]
    Io(String),
}

impl From<std::io::Error> for QuickGifError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

impl From<serde_json::Error> for QuickGifError {
    fn from(value: serde_json::Error) -> Self {
        Self::ProbeParse(value.to_string())
    }
}

impl From<anyhow::Error> for QuickGifError {
    fn from(value: anyhow::Error) -> Self {
        Self::Io(value.to_string())
    }
}

pub type QuickGifResult<T> = Result<T, QuickGifError>;
