use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use serde::{Deserialize, Serialize};

use crate::{
    error_handler::{QuickGifError, QuickGifResult},
    ffmpeg_locator,
    settings::{load_settings, resolve_output_dir, AppSettings},
    video_probe::validate_video_path,
};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFrameImageRequest {
    pub input_path: String,
    pub timestamp: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFrameImageResult {
    pub output_path: String,
    pub folder_path: String,
}

pub fn export_frame_image(
    app: &tauri::AppHandle,
    request: ExportFrameImageRequest,
) -> QuickGifResult<ExportFrameImageResult> {
    let input_path = PathBuf::from(&request.input_path);
    validate_video_path(&input_path)?;

    if !request.timestamp.is_finite() || request.timestamp < 0.0 {
        return Err(QuickGifError::InvalidRange);
    }

    let ffmpeg = ffmpeg_locator::resolve_tool(app, "ffmpeg")?;
    let settings = load_settings(app)?;
    let output_path = default_frame_output_path(&input_path, request.timestamp, &settings);
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let output = Command::new(ffmpeg)
        .args([os("-hide_banner"), os("-v"), os("error")])
        .args(build_frame_export_args(
            &input_path,
            &output_path,
            request.timestamp,
        ))
        .output()?;

    if !output.status.success() {
        return Err(QuickGifError::ToolFailed {
            tool: "ffmpeg".to_string(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    let folder_path = output_path
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .to_string_lossy()
        .to_string();

    Ok(ExportFrameImageResult {
        output_path: output_path.to_string_lossy().to_string(),
        folder_path,
    })
}

fn build_frame_export_args(input_path: &Path, output_path: &Path, timestamp: f64) -> Vec<OsString> {
    vec![
        os("-y"),
        os("-ss"),
        os(format_seconds(timestamp)),
        os("-i"),
        input_path.as_os_str().to_os_string(),
        os("-frames:v"),
        os("1"),
        os("-update"),
        os("1"),
        output_path.as_os_str().to_os_string(),
    ]
}

fn default_frame_output_path(input_path: &Path, timestamp: f64, settings: &AppSettings) -> PathBuf {
    let stem = input_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("quickgif");
    let parent = resolve_output_dir(input_path, settings);
    let timestamp = format_timestamp_slug(timestamp);
    let mut candidate = parent.join(format!("{stem}_{timestamp}.png"));
    let mut index = 1;

    while candidate.exists() {
        candidate = parent.join(format!("{stem}_{timestamp}-{index}.png"));
        index += 1;
    }

    candidate
}

fn format_timestamp_slug(value: f64) -> String {
    let total_millis = (value.max(0.0) * 1000.0).round() as u64;
    let millis = total_millis % 1000;
    let total_seconds = total_millis / 1000;
    let seconds = total_seconds % 60;
    let total_minutes = total_seconds / 60;
    let minutes = total_minutes % 60;
    let hours = total_minutes / 60;

    format!("{hours:02}-{minutes:02}-{seconds:02}.{millis:03}")
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
    fn formats_frame_output_path_with_timestamp() {
        let output = default_frame_output_path(
            Path::new("/tmp/中文 视频.mp4"),
            83.456,
            &AppSettings::default(),
        );

        assert_eq!(output, PathBuf::from("/tmp/中文 视频_00-01-23.456.png"));
    }

    #[test]
    fn builds_frame_export_args_without_shell_joining() {
        let input = Path::new("/tmp/中文 视频.mp4");
        let output = Path::new("/tmp/中文 视频_00-00-03.250.png");

        let args = build_frame_export_args(input, output, 3.25);

        assert_eq!(args[0], "-y");
        assert_eq!(args[1], "-ss");
        assert_eq!(args[2], "3.250");
        assert_eq!(args[4], input.as_os_str());
        assert!(args.iter().any(|arg| arg == "-frames:v"));
        assert_eq!(
            args.last().map(|arg| arg.as_os_str()),
            Some(output.as_os_str())
        );
    }

    #[test]
    fn writes_frame_image_to_default_export_directory_when_set() {
        let settings = AppSettings {
            language: "en".to_string(),
            default_export_dir: Some("/tmp/exports".to_string()),
            shortcuts: Default::default(),
        };

        assert_eq!(
            default_frame_output_path(Path::new("/tmp/source/clip.mp4"), 3.25, &settings),
            PathBuf::from("/tmp/exports/clip_00-00-03.250.png")
        );
    }
}
