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

use crate::{
    error_handler::{QuickGifError, QuickGifResult},
    ffmpeg_locator,
    settings::{load_settings, resolve_output_dir, AppSettings},
    video_probe::validate_video_path,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportWebpRequest {
    pub input_path: String,
    pub start_time: f64,
    pub end_time: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportWebpResult {
    pub output_path: String,
    pub folder_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportProgress {
    progress: u8,
    stage: String,
}

pub fn export_webp(
    app: &AppHandle,
    request: ExportWebpRequest,
) -> QuickGifResult<ExportWebpResult> {
    let input_path = PathBuf::from(&request.input_path);
    validate_video_path(&input_path)?;

    if request.end_time <= request.start_time {
        return Err(QuickGifError::InvalidRange);
    }

    let ffmpeg = ffmpeg_locator::resolve_tool(app, "ffmpeg")?;
    ensure_webp_encoder(&ffmpeg)?;
    let settings = load_settings(app)?;
    let output_path = default_output_path(&input_path, &settings);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    run_ffmpeg_with_progress(
        app,
        &ffmpeg,
        build_webp_export_args(
            &input_path,
            &output_path,
            request.start_time,
            request.end_time,
        ),
        request.end_time - request.start_time,
    )?;

    emit_progress(app, 100, "webpDone");

    let folder_path = output_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .to_string_lossy()
        .to_string();

    Ok(ExportWebpResult {
        output_path: output_path.to_string_lossy().to_string(),
        folder_path,
    })
}

fn build_webp_export_args(
    input_path: &Path,
    output_path: &Path,
    start_time: f64,
    end_time: f64,
) -> Vec<OsString> {
    vec![
        os("-y"),
        os("-ss"),
        os(format_seconds(start_time)),
        os("-t"),
        os(format_seconds(end_time - start_time)),
        os("-i"),
        input_path.as_os_str().to_os_string(),
        os("-map"),
        os("0:v:0"),
        os("-an"),
        os("-vf"),
        os("fps=15,scale=720:720:force_original_aspect_ratio=decrease:flags=lanczos"),
        os("-c:v"),
        os("libwebp_anim"),
        os("-lossless"),
        os("0"),
        os("-compression_level"),
        os("6"),
        os("-q:v"),
        os("78"),
        os("-loop"),
        os("0"),
        output_path.as_os_str().to_os_string(),
    ]
}

fn run_ffmpeg_with_progress(
    app: &AppHandle,
    ffmpeg: &Path,
    args: Vec<OsString>,
    duration: f64,
) -> QuickGifResult<()> {
    emit_progress(app, 3, "webpPreparing");

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
    let mut last_progress = 3;
    for line in reader.lines() {
        let line = line?;
        if let Some(seconds) = parse_progress_seconds(&line) {
            let progress = map_progress(seconds, duration);
            if progress > last_progress {
                last_progress = progress;
                emit_progress(app, progress, "webpEncoding");
            }
        }
    }

    let status = child.wait()?;
    let stderr = stderr_handle.join().unwrap_or_default();
    if status.success() {
        return Ok(());
    }

    Err(QuickGifError::ToolFailed {
        tool: "ffmpeg".to_string(),
        message: stderr.trim().to_string(),
    })
}

fn ensure_webp_encoder(ffmpeg: &Path) -> QuickGifResult<()> {
    let output = Command::new(ffmpeg)
        .args([os("-hide_banner"), os("-encoders")])
        .output()?;

    let encoders = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if output.status.success() && encoders.contains("libwebp_anim") {
        return Ok(());
    }

    Err(QuickGifError::ToolFailed {
        tool: "ffmpeg".to_string(),
        message: "当前内置 FFmpeg 不支持动画 WebP 编码，请使用包含 libwebp_anim 的 FFmpeg。"
            .to_string(),
    })
}

fn default_output_path(input_path: &Path, settings: &AppSettings) -> PathBuf {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("quickgif");
    let parent = resolve_output_dir(input_path, settings);
    let mut candidate = parent.join(format!("{stem}.webp"));
    let mut index = 1;

    while candidate.exists() {
        candidate = parent.join(format!("{stem}-{index}.webp"));
        index += 1;
    }

    candidate
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

fn map_progress(seconds: f64, duration: f64) -> u8 {
    if duration <= 0.0 {
        return 3;
    }
    (3.0 + 94.0 * (seconds / duration).clamp(0.0, 1.0)).round() as u8
}

fn emit_progress(app: &AppHandle, progress: u8, stage: &str) {
    let _ = app.emit(
        "webp-export-progress",
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
    use std::path::{Path, PathBuf};

    #[test]
    fn writes_webp_to_default_export_directory_when_set() {
        let settings = AppSettings {
            language: "en".to_string(),
            default_export_dir: Some("/tmp/exports".to_string()),
            shortcuts: Default::default(),
        };

        assert_eq!(
            default_output_path(Path::new("/tmp/source/clip.mov"), &settings),
            PathBuf::from("/tmp/exports/clip.webp")
        );
    }

    #[test]
    fn builds_animated_webp_args_without_shell_joining() {
        let input = Path::new("/tmp/中文 视频.mov");
        let output = Path::new("/tmp/exports/中文 视频.webp");

        let args = build_webp_export_args(input, output, 1.25, 4.75);

        assert_eq!(args[0], "-y");
        assert_eq!(args[1], "-ss");
        assert_eq!(args[2], "1.250");
        assert!(args.iter().any(|arg| arg == "-c:v"));
        assert!(args.iter().any(|arg| arg == "libwebp_anim"));
        assert!(args.iter().any(|arg| arg == "-loop"));
        assert!(args.iter().any(|arg| arg == "0"));
        assert!(args
            .iter()
            .any(|arg| arg.to_string_lossy().contains("fps=15")));
        assert_eq!(
            args.last().map(|arg| arg.as_os_str()),
            Some(output.as_os_str())
        );
    }
}
