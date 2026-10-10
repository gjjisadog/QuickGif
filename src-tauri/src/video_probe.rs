use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::{
    error_handler::{QuickGifError, QuickGifResult},
    ffmpeg_locator,
};

const SUPPORTED_EXTENSIONS: &[&str] = &["mp4", "mov", "avi", "mkv", "webm"];
/// Still side of an Apple Live Photo pair — we resolve the sibling motion video.
const LIVE_PHOTO_STILL_EXTENSIONS: &[&str] = &["jpg", "jpeg", "heic", "heif", "png"];
const LIVE_PHOTO_VIDEO_CANDIDATES: &[&str] = &["MOV", "mov", "MP4", "mp4"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoInfo {
    pub path: String,
    pub file_name: String,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub frame_rate: f64,
    pub codec: String,
    pub total_frames: u64,
}

#[derive(Debug, Deserialize)]
struct FfprobeOutput {
    streams: Vec<FfprobeStream>,
    format: Option<FfprobeFormat>,
}

#[derive(Debug, Deserialize)]
struct FfprobeFormat {
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FfprobeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    avg_frame_rate: Option<String>,
    r_frame_rate: Option<String>,
    duration: Option<String>,
    nb_frames: Option<String>,
}

pub fn probe_video(app: &AppHandle, path: String) -> QuickGifResult<VideoInfo> {
    let path = resolve_import_media_path(PathBuf::from(path))?;
    validate_video_path(&path)?;

    let ffprobe = ffmpeg_locator::resolve_tool(app, "ffprobe")?;
    let output = Command::new(ffprobe)
        .args([
            OsStr::new("-v"),
            OsStr::new("error"),
            OsStr::new("-print_format"),
            OsStr::new("json"),
            OsStr::new("-show_streams"),
            OsStr::new("-show_format"),
        ])
        .arg(&path)
        .output()?;

    if !output.status.success() {
        return Err(QuickGifError::ToolFailed {
            tool: "ffprobe".to_string(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    parse_probe_json(&path, &output.stdout)
}

/// Resolve Apple Live Photo stills to their paired motion video when present.
pub fn resolve_import_media_path(path: PathBuf) -> QuickGifResult<PathBuf> {
    if !path.is_file() {
        return Err(QuickGifError::FileNotFound(path));
    }

    let extension = path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_lowercase();

    if SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
        return Ok(path);
    }

    if !LIVE_PHOTO_STILL_EXTENSIONS.contains(&extension.as_str()) {
        return Err(QuickGifError::UnsupportedFormat(extension));
    }

    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .and_then(OsStr::to_str)
        .ok_or_else(|| QuickGifError::UnsupportedFormat(extension.clone()))?;

    for candidate_ext in LIVE_PHOTO_VIDEO_CANDIDATES {
        let candidate = parent.join(format!("{stem}.{candidate_ext}"));
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err(QuickGifError::Io(format!(
        "已识别为可能的实况照片静态图，但未找到同名配对视频（{stem}.MOV / .MP4）。请选择配对的 .MOV，或同时保留 JPG/HEIC 与 MOV。"
    )))
}

pub fn validate_video_path(path: &Path) -> QuickGifResult<()> {
    if !path.is_file() {
        return Err(QuickGifError::FileNotFound(path.to_path_buf()));
    }

    let extension = path
        .extension()
        .and_then(OsStr::to_str)
        .unwrap_or_default()
        .to_lowercase();

    if !SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
        return Err(QuickGifError::UnsupportedFormat(extension));
    }

    Ok(())
}

fn parse_probe_json(path: &Path, bytes: &[u8]) -> QuickGifResult<VideoInfo> {
    let parsed: FfprobeOutput = serde_json::from_slice(bytes)?;
    let stream = parsed
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("video"))
        .ok_or_else(|| QuickGifError::ProbeParse("未找到视频流".to_string()))?;

    let duration = stream
        .duration
        .as_deref()
        .or(parsed.format.and_then(|format| format.duration).as_deref())
        .and_then(parse_f64)
        .unwrap_or(0.0);
    let frame_rate = stream
        .avg_frame_rate
        .as_deref()
        .or(stream.r_frame_rate.as_deref())
        .and_then(parse_fraction)
        .unwrap_or(0.0);
    let total_frames = stream
        .nb_frames
        .as_deref()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or_else(|| (duration * frame_rate).round().max(0.0) as u64);

    Ok(VideoInfo {
        path: path.to_string_lossy().to_string(),
        file_name: path
            .file_name()
            .and_then(OsStr::to_str)
            .unwrap_or("video")
            .to_string(),
        duration,
        width: stream.width.unwrap_or(0),
        height: stream.height.unwrap_or(0),
        frame_rate,
        codec: stream
            .codec_name
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        total_frames,
    })
}

fn parse_f64(value: &str) -> Option<f64> {
    value.parse::<f64>().ok().filter(|value| value.is_finite())
}

pub fn parse_fraction(value: &str) -> Option<f64> {
    if let Some((left, right)) = value.split_once('/') {
        let numerator = left.parse::<f64>().ok()?;
        let denominator = right.parse::<f64>().ok()?;
        if denominator == 0.0 {
            return None;
        }
        Some(numerator / denominator)
    } else {
        parse_f64(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fraction_frame_rates() {
        assert_eq!(parse_fraction("24/1"), Some(24.0));
        assert!((parse_fraction("30000/1001").unwrap() - 29.970).abs() < 0.01);
        assert_eq!(parse_fraction("0/0"), None);
    }

    #[test]
    fn rejects_unsupported_extensions() {
        let path = PathBuf::from("clip.txt");
        let result = validate_extension_only(&path);
        assert!(matches!(result, Err(QuickGifError::UnsupportedFormat(_))));
    }

    #[test]
    fn live_photo_still_extensions_are_recognized() {
        assert!(LIVE_PHOTO_STILL_EXTENSIONS.contains(&"heic"));
        assert!(LIVE_PHOTO_STILL_EXTENSIONS.contains(&"jpg"));
    }

    fn validate_extension_only(path: &Path) -> QuickGifResult<()> {
        let extension = path
            .extension()
            .and_then(OsStr::to_str)
            .unwrap_or_default()
            .to_lowercase();
        if !SUPPORTED_EXTENSIONS.contains(&extension.as_str()) {
            return Err(QuickGifError::UnsupportedFormat(extension));
        }
        Ok(())
    }
}
