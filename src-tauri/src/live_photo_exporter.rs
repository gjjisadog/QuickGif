use std::{
    ffi::OsString,
    fs,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::{
    apple_live_photo_meta,
    error_handler::{QuickGifError, QuickGifResult},
    ffmpeg_locator,
    settings::{load_settings, resolve_output_dir, AppSettings},
    temp_manager,
    video_probe::validate_video_path,
};

/// Live Photos are short motion clips; longer ranges hurt Photos compatibility.
const MIN_DURATION_SECS: f64 = 0.5;
const MAX_DURATION_SECS: f64 = 6.0;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportLivePhotoRequest {
    pub input_path: String,
    pub start_time: f64,
    pub end_time: f64,
    /// Absolute timestamp in source video for the still (key photo).
    /// When omitted or out of range, the clip midpoint is used.
    pub still_time: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportLivePhotoResult {
    pub output_path: String,
    pub image_path: String,
    pub video_path: String,
    pub folder_path: String,
    pub content_identifier: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportProgress {
    progress: u8,
    stage: String,
}

pub fn export_live_photo(
    app: &AppHandle,
    request: ExportLivePhotoRequest,
) -> QuickGifResult<ExportLivePhotoResult> {
    let input_path = PathBuf::from(&request.input_path);
    validate_video_path(&input_path)?;

    if !request.start_time.is_finite()
        || !request.end_time.is_finite()
        || request.end_time <= request.start_time
    {
        return Err(QuickGifError::InvalidRange);
    }

    let duration = request.end_time - request.start_time;
    if duration < MIN_DURATION_SECS || duration > MAX_DURATION_SECS {
        return Err(QuickGifError::Io(format!(
            "实况照片时长需在 {MIN_DURATION_SECS:.1}–{MAX_DURATION_SECS:.0} 秒之间（当前 {duration:.2} 秒）。请缩短或延长选区。"
        )));
    }

    let still_time = resolve_still_time(&request);
    let content_id = Uuid::new_v4().to_string().to_uppercase();
    let ffmpeg = ffmpeg_locator::resolve_tool(app, "ffmpeg")?;
    let settings = load_settings(app)?;
    let (image_path, video_path) = default_pair_paths(&input_path, &settings);
    if let Some(parent) = image_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let temp_dir = temp_manager::create_temp_dir("livephoto")?;
    let temp_video = temp_dir.join("pair.MOV");
    let temp_image = temp_dir.join("pair.JPG");

    let result = (|| -> QuickGifResult<ExportLivePhotoResult> {
        emit_progress(app, 4, "livePhotoPreparing");

        // 1) Motion video with pairing tags (keep audio when present).
        run_ffmpeg_with_progress(
            app,
            &ffmpeg,
            build_live_video_args(
                &input_path,
                &temp_video,
                request.start_time,
                duration,
                &content_id,
                still_time - request.start_time,
            ),
            duration,
            "livePhotoEncodingVideo",
            5,
            70,
        )?;

        emit_progress(app, 74, "livePhotoExtractingStill");

        // 2) Key photo JPEG from the same absolute still time.
        run_ffmpeg_simple(
            &ffmpeg,
            build_still_args(&input_path, &temp_image, still_time),
        )?;

        emit_progress(app, 88, "livePhotoWritingMetadata");
        apple_live_photo_meta::inject_content_identifier(&temp_image, &content_id)?;

        // 3) Promote paired files with matching basenames.
        fs::copy(&temp_image, &image_path)?;
        fs::copy(&temp_video, &video_path)?;

        emit_progress(app, 100, "livePhotoDone");

        let folder_path = image_path
            .parent()
            .unwrap_or_else(|| Path::new(""))
            .to_string_lossy()
            .to_string();

        Ok(ExportLivePhotoResult {
            output_path: image_path.to_string_lossy().to_string(),
            image_path: image_path.to_string_lossy().to_string(),
            video_path: video_path.to_string_lossy().to_string(),
            folder_path,
            content_identifier: content_id,
        })
    })();

    temp_manager::cleanup_dir(temp_dir);
    result
}

fn resolve_still_time(request: &ExportLivePhotoRequest) -> f64 {
    let midpoint = (request.start_time + request.end_time) / 2.0;
    match request.still_time {
        Some(value)
            if value.is_finite() && value >= request.start_time && value <= request.end_time =>
        {
            value
        }
        _ => midpoint,
    }
}

fn build_live_video_args(
    input_path: &Path,
    output_path: &Path,
    start_time: f64,
    duration: f64,
    content_id: &str,
    still_offset: f64,
) -> Vec<OsString> {
    let still_offset = still_offset.clamp(0.0, duration);
    // still-image-time is best-effort metadata; Content Identifier is the pairing key.
    let still_tag = format!("{still_offset:.3}");

    vec![
        os("-y"),
        os("-ss"),
        os(format_seconds(start_time)),
        os("-t"),
        os(format_seconds(duration)),
        os("-i"),
        input_path.as_os_str().to_os_string(),
        os("-map"),
        os("0:v:0"),
        os("-map"),
        os("0:a:0?"),
        os("-vf"),
        os("scale=trunc(iw/2)*2:trunc(ih/2)*2"),
        os("-c:v"),
        os("libx264"),
        os("-preset"),
        os("veryfast"),
        os("-crf"),
        os("18"),
        os("-pix_fmt"),
        os("yuv420p"),
        os("-c:a"),
        os("aac"),
        os("-b:a"),
        os("128k"),
        os("-movflags"),
        os("+faststart+use_metadata_tags"),
        os("-metadata"),
        os(format!("content.identifier={content_id}")),
        os("-metadata"),
        os(format!(
            "com.apple.quicktime.content.identifier={content_id}"
        )),
        os("-metadata"),
        os(format!("com.apple.quicktime.still-image-time={still_tag}")),
        os("-f"),
        os("mov"),
        output_path.as_os_str().to_os_string(),
    ]
}

fn build_still_args(input_path: &Path, output_path: &Path, still_time: f64) -> Vec<OsString> {
    vec![
        os("-y"),
        os("-ss"),
        os(format_seconds(still_time)),
        os("-i"),
        input_path.as_os_str().to_os_string(),
        os("-frames:v"),
        os("1"),
        os("-q:v"),
        os("2"),
        os("-f"),
        os("image2"),
        output_path.as_os_str().to_os_string(),
    ]
}

fn default_pair_paths(input_path: &Path, settings: &AppSettings) -> (PathBuf, PathBuf) {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("quickgif");
    let parent = resolve_output_dir(input_path, settings);
    let base = format!("{stem}_Live");
    let mut image = parent.join(format!("{base}.JPG"));
    let mut video = parent.join(format!("{base}.MOV"));
    let mut index = 1;

    while image.exists() || video.exists() {
        let indexed = format!("{base}-{index}");
        image = parent.join(format!("{indexed}.JPG"));
        video = parent.join(format!("{indexed}.MOV"));
        index += 1;
    }

    (image, video)
}

fn run_ffmpeg_simple(ffmpeg: &Path, args: Vec<OsString>) -> QuickGifResult<()> {
    let output = Command::new(ffmpeg)
        .args([os("-hide_banner"), os("-v"), os("error")])
        .args(args)
        .output()?;

    if output.status.success() {
        return Ok(());
    }

    Err(QuickGifError::ToolFailed {
        tool: "ffmpeg".to_string(),
        message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

fn run_ffmpeg_with_progress(
    app: &AppHandle,
    ffmpeg: &Path,
    args: Vec<OsString>,
    duration: f64,
    stage: &str,
    progress_start: u8,
    progress_end: u8,
) -> QuickGifResult<()> {
    let mut command = Command::new(ffmpeg);
    command.args([
        os("-hide_banner"),
        os("-v"),
        os("error"),
        os("-nostats"),
        os("-progress"),
        os("pipe:1"),
    ]);
    command.args(args);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = command.spawn()?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| QuickGifError::ToolFailed {
            tool: "ffmpeg".to_string(),
            message: "无法读取 FFmpeg 进度输出".to_string(),
        })?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| QuickGifError::ToolFailed {
            tool: "ffmpeg".to_string(),
            message: "无法读取 FFmpeg 错误输出".to_string(),
        })?;

    let stderr_handle = thread::spawn(move || {
        let mut reader = BufReader::new(stderr);
        let mut output = String::new();
        let _ = reader.read_to_string(&mut output);
        output
    });

    let reader = BufReader::new(stdout);
    let mut last_progress = progress_start;
    for line in reader.lines() {
        let line = line?;
        if let Some(seconds) = parse_progress_seconds(&line) {
            let progress = map_progress(seconds, duration, progress_start, progress_end);
            if progress > last_progress {
                last_progress = progress;
                emit_progress(app, progress, stage);
            }
        }
    }

    let status = child.wait()?;
    let stderr = stderr_handle.join().unwrap_or_default();
    if status.success() {
        return Ok(());
    }

    // Retry without audio map if source has no audio stream.
    if stderr.to_lowercase().contains("stream map")
        || stderr.to_lowercase().contains("matches no streams")
    {
        return Err(QuickGifError::ToolFailed {
            tool: "ffmpeg".to_string(),
            message: format!("导出实况视频失败：{}", stderr.trim()),
        });
    }

    Err(QuickGifError::ToolFailed {
        tool: "ffmpeg".to_string(),
        message: stderr.trim().to_string(),
    })
}

fn parse_progress_seconds(line: &str) -> Option<f64> {
    let (key, value) = line.split_once('=')?;
    match key {
        "out_time_ms" | "out_time_us" => value.parse::<f64>().ok().map(|value| value / 1_000_000.0),
        "out_time" => parse_timestamp_seconds(value),
        _ => None,
    }
    .filter(|value| value.is_finite() && *value >= 0.0)
}

fn parse_timestamp_seconds(value: &str) -> Option<f64> {
    let mut parts = value.split(':');
    let hours = parts.next()?.parse::<f64>().ok()?;
    let minutes = parts.next()?.parse::<f64>().ok()?;
    let seconds = parts.next()?.parse::<f64>().ok()?;
    Some(hours * 3600.0 + minutes * 60.0 + seconds)
}

fn map_progress(seconds: f64, duration: f64, start: u8, end: u8) -> u8 {
    if duration <= 0.0 || end <= start {
        return start;
    }
    let ratio = (seconds / duration).clamp(0.0, 1.0);
    (f64::from(start) + f64::from(end - start) * ratio).round() as u8
}

fn emit_progress(app: &AppHandle, progress: u8, stage: &str) {
    let _ = app.emit(
        "livephoto-export-progress",
        ExportProgress {
            progress,
            stage: stage.to_string(),
        },
    );
}

fn format_seconds(value: f64) -> String {
    format!("{value:.3}")
}

fn os(value: impl Into<OsString>) -> OsString {
    value.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::AppSettings;

    #[test]
    fn still_time_defaults_to_midpoint() {
        let request = ExportLivePhotoRequest {
            input_path: "/tmp/a.mp4".into(),
            start_time: 1.0,
            end_time: 3.0,
            still_time: None,
        };
        assert!((resolve_still_time(&request) - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn still_time_uses_playhead_when_in_range() {
        let request = ExportLivePhotoRequest {
            input_path: "/tmp/a.mp4".into(),
            start_time: 1.0,
            end_time: 3.0,
            still_time: Some(1.25),
        };
        assert!((resolve_still_time(&request) - 1.25).abs() < f64::EPSILON);
    }

    #[test]
    fn pair_paths_use_live_suffix_and_match_stem() {
        let settings = AppSettings {
            language: "zh-CN".into(),
            default_export_dir: Some("/tmp/exports".into()),
            shortcuts: Default::default(),
        };
        let (image, video) = default_pair_paths(Path::new("/tmp/source/demo.mp4"), &settings);
        assert_eq!(image.extension().and_then(|v| v.to_str()), Some("JPG"));
        assert_eq!(video.extension().and_then(|v| v.to_str()), Some("MOV"));
        assert_eq!(
            image.file_stem().and_then(|v| v.to_str()),
            video.file_stem().and_then(|v| v.to_str())
        );
        assert!(image
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap()
            .contains("_Live"));
    }

    #[test]
    fn video_args_include_content_identifier_and_optional_audio() {
        let args = build_live_video_args(
            Path::new("/tmp/中文.mp4"),
            Path::new("/tmp/out.MOV"),
            0.5,
            2.0,
            "ABCD-EFGH",
            1.0,
        );
        let joined: Vec<String> = args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect();
        assert!(joined.iter().any(|a| a.contains("content.identifier=ABCD-EFGH")));
        assert!(joined.iter().any(|a| a == "0:a:0?"));
        assert!(joined.iter().any(|a| a == "mov"));
    }
}
