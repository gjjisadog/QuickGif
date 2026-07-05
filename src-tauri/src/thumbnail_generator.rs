use std::{ffi::OsString, path::Path, process::Command};

use serde::Serialize;
use tauri::AppHandle;

use crate::{
    error_handler::{QuickGifError, QuickGifResult},
    ffmpeg_locator, temp_manager,
    video_probe::validate_video_path,
};

const MAX_THUMBNAILS: u32 = 48;
const MAX_DETAIL_THUMBNAILS: u32 = 24;
const MIN_OVERVIEW_INTERVAL: f64 = 0.5;
const MIN_DETAIL_INTERVAL: f64 = 0.2;
const OVERVIEW_THUMBNAIL_WIDTH: u32 = 96;
const DETAIL_THUMBNAIL_WIDTH: u32 = 132;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThumbnailResult {
    pub directory: String,
    pub files: Vec<String>,
    pub start_time: f64,
    pub interval_seconds: f64,
}

pub fn generate_thumbnails(
    app: &AppHandle,
    input_path: &Path,
    duration: f64,
) -> QuickGifResult<ThumbnailResult> {
    generate_thumbnail_range(
        app,
        input_path,
        0.0,
        duration,
        MAX_THUMBNAILS,
        MIN_OVERVIEW_INTERVAL,
        OVERVIEW_THUMBNAIL_WIDTH,
        "fast_bilinear",
    )
}

pub fn generate_detail_thumbnails(
    app: &AppHandle,
    input_path: &Path,
    start_time: f64,
    duration: f64,
) -> QuickGifResult<ThumbnailResult> {
    generate_thumbnail_range(
        app,
        input_path,
        start_time,
        duration,
        MAX_DETAIL_THUMBNAILS,
        MIN_DETAIL_INTERVAL,
        DETAIL_THUMBNAIL_WIDTH,
        "bilinear",
    )
}

fn generate_thumbnail_range(
    app: &AppHandle,
    input_path: &Path,
    start_time: f64,
    duration: f64,
    max_thumbnails: u32,
    min_interval: f64,
    width: u32,
    scale_flags: &str,
) -> QuickGifResult<ThumbnailResult> {
    validate_video_path(input_path)?;
    if duration <= 0.0 || start_time < 0.0 {
        return Err(QuickGifError::InvalidRange);
    }

    let ffmpeg = ffmpeg_locator::resolve_tool(app, "ffmpeg")?;
    let temp_dir = temp_manager::create_temp_dir("thumbs")?;
    let interval = thumbnail_interval(duration, max_thumbnails, min_interval);
    let pattern = temp_dir.join("thumb-%04d.jpg");
    let vf = format!("fps=1/{interval:.3},scale={width}:-1:flags={scale_flags}");

    let output = Command::new(ffmpeg)
        .args([
            OsString::from("-y"),
            OsString::from("-hide_banner"),
            OsString::from("-ss"),
            OsString::from(format!("{start_time:.3}")),
            OsString::from("-t"),
            OsString::from(format!("{duration:.3}")),
            OsString::from("-i"),
        ])
        .arg(input_path)
        .args([
            OsString::from("-an"),
            OsString::from("-sn"),
            OsString::from("-vf"),
            OsString::from(vf),
            OsString::from("-frames:v"),
            OsString::from(max_thumbnails.to_string()),
            OsString::from("-q:v"),
            OsString::from("8"),
        ])
        .arg(&pattern)
        .output()?;

    if !output.status.success() {
        return Err(QuickGifError::ToolFailed {
            tool: "ffmpeg".to_string(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    let mut files = std::fs::read_dir(&temp_dir)?
        .flatten()
        .map(|entry| entry.path().to_string_lossy().to_string())
        .collect::<Vec<_>>();
    files.sort();

    Ok(ThumbnailResult {
        directory: temp_dir.to_string_lossy().to_string(),
        files,
        start_time,
        interval_seconds: interval,
    })
}

pub fn thumbnail_interval(duration: f64, max_thumbnails: u32, min_interval: f64) -> f64 {
    if duration <= 0.0 {
        return 1.0;
    }
    let thumbnail_count = max_thumbnails.max(1);
    (duration / f64::from(thumbnail_count)).max(min_interval)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caps_thumbnail_count_by_increasing_interval() {
        assert_eq!(thumbnail_interval(20.0, 80, 0.5), 0.5);
        assert_eq!(thumbnail_interval(160.0, 80, 0.5), 2.0);
    }

    #[test]
    fn uses_one_second_interval_for_missing_duration() {
        assert_eq!(thumbnail_interval(0.0, 80, 0.5), 1.0);
        assert_eq!(thumbnail_interval(-1.0, 80, 0.5), 1.0);
    }

    #[test]
    fn supports_dense_detail_intervals_for_selected_ranges() {
        assert_eq!(thumbnail_interval(12.0, MAX_DETAIL_THUMBNAILS, 0.2), 0.5);
        assert_eq!(
            thumbnail_interval(32.0, MAX_DETAIL_THUMBNAILS, 0.2),
            32.0 / 24.0
        );
    }

    #[test]
    fn overview_uses_fewer_positioning_thumbnails() {
        assert_eq!(thumbnail_interval(12.0, MAX_THUMBNAILS, 0.5), 0.5);
        assert_eq!(thumbnail_interval(120.0, MAX_THUMBNAILS, 0.5), 2.5);
    }
}
