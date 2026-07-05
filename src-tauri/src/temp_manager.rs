use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::error_handler::QuickGifResult;

const TEMP_PREFIX: &str = "quickgif-";

pub fn create_temp_dir(label: &str) -> QuickGifResult<PathBuf> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let dir = std::env::temp_dir().join(format!("{TEMP_PREFIX}{label}-{stamp}"));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn cleanup_dir(dir: PathBuf) {
    let _ = fs::remove_dir_all(dir);
}

pub fn cleanup_stale_temp_dirs() {
    let Ok(entries) = fs::read_dir(std::env::temp_dir()) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if name.starts_with(TEMP_PREFIX) {
            let _ = fs::remove_dir_all(path);
        }
    }
}
