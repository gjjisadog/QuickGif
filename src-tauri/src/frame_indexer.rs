use std::{path::Path, process::Command};

use serde::Serialize;
use tauri::AppHandle;

use crate::{
    error_handler::{QuickGifError, QuickGifResult},
    ffmpeg_locator,
    video_probe::validate_video_path,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FramePoint {
    pub frame: u64,
    pub timestamp: f64,
}

pub fn build_frame_index(app: &AppHandle, input_path: &Path) -> QuickGifResult<Vec<FramePoint>> {
    validate_video_path(input_path)?;
    let ffprobe = ffmpeg_locator::resolve_tool(app, "ffprobe")?;
    let output = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "frame=best_effort_timestamp_time",
            "-of",
            "csv=p=0",
        ])
        .arg(input_path)
        .output()?;

    if !output.status.success() {
        return Err(QuickGifError::ToolFailed {
            tool: "ffprobe".to_string(),
            message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_frame_timestamps(&stdout))
}

pub fn parse_frame_timestamps(stdout: &str) -> Vec<FramePoint> {
    stdout
        .lines()
        // Some codecs append side-data text to the CSV row (for example an
        // unregistered SEI message).  The timestamp is still the first field;
        // parsing the whole row silently dropped those otherwise valid frames.
        .filter_map(|line| line.split(',').next()?.trim().parse::<f64>().ok())
        .enumerate()
        .map(|(index, timestamp)| FramePoint {
            frame: index as u64,
            timestamp,
        })
        .collect()
}

#[cfg(test)]
fn nearest_frame(frames: &[FramePoint], timestamp: f64) -> Option<&FramePoint> {
    frames.iter().min_by(|left, right| {
        let left_delta = (left.timestamp - timestamp).abs();
        let right_delta = (right.timestamp - timestamp).abs();
        left_delta.total_cmp(&right_delta)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frame_timestamp_csv() {
        let frames = parse_frame_timestamps("0.000000\n0.041667\n");
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[1].frame, 1);
        assert!((frames[1].timestamp - 0.041667).abs() < 0.0001);
    }

    #[test]
    fn keeps_frames_with_ffprobe_side_data_in_csv_row() {
        let frames =
            parse_frame_timestamps("0.000000,H.264 User Data Unregistered SEI message\n0.041667\n");

        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].frame, 0);
        assert_eq!(frames[0].timestamp, 0.0);
    }

    #[test]
    fn finds_nearest_frame_by_timestamp() {
        let frames = parse_frame_timestamps("0.000000\n0.041667\n0.083333\n");
        let frame = nearest_frame(&frames, 0.05).unwrap();
        assert_eq!(frame.frame, 1);
    }
}
