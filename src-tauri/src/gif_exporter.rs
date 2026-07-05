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
    temp_manager,
    video_probe::validate_video_path,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportGifRequest {
    pub input_path: String,
    pub start_time: f64,
    pub end_time: f64,
    pub width: ExportWidth,
    pub frame_rate: ExportFrameRate,
    pub quality: ExportQuality,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportWidth {
    Original,
    W720,
    W480,
    W320,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportFrameRate {
    Original,
    Fps24,
    Fps15,
    Fps10,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportQuality {
    Small,
    Standard,
    High,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportGifResult {
    pub output_path: String,
    pub folder_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportProgress {
    progress: u8,
    stage: String,
}

pub fn export_gif(app: &AppHandle, request: ExportGifRequest) -> QuickGifResult<ExportGifResult> {
    let input_path = PathBuf::from(&request.input_path);
    validate_video_path(&input_path)?;

    if request.end_time <= request.start_time {
        return Err(QuickGifError::InvalidRange);
    }

    let ffmpeg = ffmpeg_locator::resolve_tool(app, "ffmpeg")?;
    let temp_dir = temp_manager::create_temp_dir("export")?;
    let palette_path = temp_dir.join("palette.png");
    let settings = load_settings(app)?;
    let output_path = default_output_path(&input_path, &settings);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let filter = build_video_filter(&request);
    let quality = quality_settings(&request.quality);

    emit_progress(app, 3, "准备导出");
    let result = export_gif_inner(
        app,
        &request,
        &ffmpeg,
        &input_path,
        &palette_path,
        &output_path,
        &filter,
        quality,
    );

    temp_manager::cleanup_dir(temp_dir);
    result?;

    emit_progress(app, 100, "导出完成");

    let folder_path = output_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .to_string_lossy()
        .to_string();

    Ok(ExportGifResult {
        output_path: output_path.to_string_lossy().to_string(),
        folder_path,
    })
}

fn export_gif_inner(
    app: &AppHandle,
    request: &ExportGifRequest,
    ffmpeg: &Path,
    input_path: &Path,
    palette_path: &Path,
    output_path: &Path,
    filter: &str,
    quality: QualitySettings,
) -> QuickGifResult<()> {
    run_palettegen(
        app,
        ffmpeg,
        input_path,
        palette_path,
        request.start_time,
        request.end_time,
        filter,
        quality.max_colors,
    )?;

    run_paletteuse(
        app,
        ffmpeg,
        input_path,
        palette_path,
        output_path,
        request.start_time,
        request.end_time,
        filter,
        quality.paletteuse,
    )?;

    emit_progress(app, 99, "收尾保存");
    Ok(())
}

fn run_palettegen(
    app: &AppHandle,
    ffmpeg: &Path,
    input_path: &Path,
    palette_path: &Path,
    start_time: f64,
    end_time: f64,
    filter: &str,
    max_colors: u16,
) -> QuickGifResult<()> {
    let filter = format!("{filter},palettegen=max_colors={max_colors}:stats_mode=diff");
    let args = vec![
        os("-y"),
        os("-ss"),
        os(format_seconds(start_time)),
        os("-to"),
        os(format_seconds(end_time)),
        os("-i"),
        input_path.as_os_str().to_os_string(),
        os("-vf"),
        os(filter),
        palette_path.as_os_str().to_os_string(),
    ];
    run_ffmpeg_with_progress(
        app,
        ffmpeg,
        args,
        end_time - start_time,
        "生成调色板",
        5,
        45,
    )
}

fn run_paletteuse(
    app: &AppHandle,
    ffmpeg: &Path,
    input_path: &Path,
    palette_path: &Path,
    output_path: &Path,
    start_time: f64,
    end_time: f64,
    filter: &str,
    paletteuse: &str,
) -> QuickGifResult<()> {
    let lavfi = format!("[0:v]{filter}[x];[x][1:v]paletteuse={paletteuse}");
    let args = vec![
        os("-y"),
        os("-ss"),
        os(format_seconds(start_time)),
        os("-to"),
        os(format_seconds(end_time)),
        os("-i"),
        input_path.as_os_str().to_os_string(),
        os("-i"),
        palette_path.as_os_str().to_os_string(),
        os("-lavfi"),
        os(lavfi),
        os("-loop"),
        os("0"),
        output_path.as_os_str().to_os_string(),
    ];
    run_ffmpeg_with_progress(app, ffmpeg, args, end_time - start_time, "生成 GIF", 45, 99)
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
    emit_progress(app, progress_start, stage);

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
            let progress = map_stage_progress(seconds, duration, progress_start, progress_end);
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

    Err(QuickGifError::ToolFailed {
        tool: "ffmpeg".to_string(),
        message: stderr.trim().to_string(),
    })
}

fn build_video_filter(request: &ExportGifRequest) -> String {
    let mut filters = Vec::new();
    if let Some(fps) = request.frame_rate.value() {
        filters.push(format!("fps={fps}"));
    }
    if let Some(width) = request.width.value() {
        filters.push(format!("scale={width}:-1:flags=lanczos"));
    }
    if filters.is_empty() {
        "scale=iw:ih:flags=lanczos".to_string()
    } else {
        filters.join(",")
    }
}

fn default_output_path(input_path: &Path, settings: &AppSettings) -> PathBuf {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("quickgif");
    let parent = resolve_output_dir(input_path, settings);
    let mut candidate = parent.join(format!("{stem}.gif"));
    let mut index = 1;

    while candidate.exists() {
        candidate = parent.join(format!("{stem}-{index}.gif"));
        index += 1;
    }

    candidate
}

fn quality_settings(quality: &ExportQuality) -> QualitySettings {
    match quality {
        ExportQuality::Small => QualitySettings {
            max_colors: 128,
            paletteuse: "dither=bayer:bayer_scale=5",
        },
        ExportQuality::Standard => QualitySettings {
            max_colors: 192,
            paletteuse: "dither=sierra2_4a",
        },
        ExportQuality::High => QualitySettings {
            max_colors: 256,
            paletteuse: "dither=sierra2_4a",
        },
    }
}

struct QualitySettings {
    max_colors: u16,
    paletteuse: &'static str,
}

impl ExportWidth {
    fn value(&self) -> Option<u16> {
        match self {
            ExportWidth::Original => None,
            ExportWidth::W720 => Some(720),
            ExportWidth::W480 => Some(480),
            ExportWidth::W320 => Some(320),
        }
    }
}

impl ExportFrameRate {
    fn value(&self) -> Option<u16> {
        match self {
            ExportFrameRate::Original => None,
            ExportFrameRate::Fps24 => Some(24),
            ExportFrameRate::Fps15 => Some(15),
            ExportFrameRate::Fps10 => Some(10),
        }
    }
}

fn emit_progress(app: &AppHandle, progress: u8, stage: &str) {
    let _ = app.emit(
        "gif-export-progress",
        ExportProgress {
            progress,
            stage: stage.to_string(),
        },
    );
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

fn map_stage_progress(seconds: f64, duration: f64, progress_start: u8, progress_end: u8) -> u8 {
    if duration <= 0.0 {
        return progress_start;
    }

    let ratio = (seconds / duration).clamp(0.0, 1.0);
    let span = f64::from(progress_end.saturating_sub(progress_start));
    (f64::from(progress_start) + span * ratio).round() as u8
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

    fn request(width: ExportWidth, frame_rate: ExportFrameRate) -> ExportGifRequest {
        ExportGifRequest {
            input_path: "clip.mp4".to_string(),
            start_time: 0.0,
            end_time: 1.0,
            width,
            frame_rate,
            quality: ExportQuality::Standard,
        }
    }

    #[test]
    fn builds_lanczos_scale_and_fps_filter() {
        let filter = build_video_filter(&request(ExportWidth::W480, ExportFrameRate::Fps15));
        assert_eq!(filter, "fps=15,scale=480:-1:flags=lanczos");
    }

    #[test]
    fn keeps_original_size_with_lanczos_noop_filter() {
        let filter = build_video_filter(&request(ExportWidth::Original, ExportFrameRate::Original));
        assert_eq!(filter, "scale=iw:ih:flags=lanczos");
    }

    #[test]
    fn parses_ffmpeg_progress_time_lines() {
        assert_eq!(parse_progress_seconds("out_time_ms=1250000"), Some(1.25));
        assert_eq!(parse_progress_seconds("out_time_us=2500000"), Some(2.5));
        assert_eq!(
            parse_progress_seconds("out_time=00:00:03.500000"),
            Some(3.5)
        );
        assert_eq!(parse_progress_seconds("progress=continue"), None);
    }

    #[test]
    fn maps_progress_into_stage_range() {
        assert_eq!(map_stage_progress(0.5, 2.0, 10, 40), 18);
        assert_eq!(map_stage_progress(3.0, 2.0, 10, 40), 40);
        assert_eq!(map_stage_progress(1.0, 0.0, 10, 40), 10);
    }

    #[test]
    fn writes_gif_to_default_export_directory_when_set() {
        let settings = AppSettings {
            language: "en".to_string(),
            default_export_dir: Some("/tmp/exports".to_string()),
            shortcuts: Default::default(),
        };

        assert_eq!(
            default_output_path(Path::new("/tmp/source/clip.mp4"), &settings),
            PathBuf::from("/tmp/exports/clip.gif")
        );
    }
}
