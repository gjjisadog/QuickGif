use std::{
    env,
    path::{Path, PathBuf},
};

use tauri::{AppHandle, Manager};

use crate::error_handler::{QuickGifError, QuickGifResult};

pub fn resolve_tool(app: &AppHandle, tool: &str) -> QuickGifResult<PathBuf> {
    if let Ok(path) = env::var(format!("QUICKGIF_{}_PATH", tool.to_uppercase())) {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
    }

    if let Ok(dir) = env::var("QUICKGIF_FFMPEG_DIR") {
        if let Some(path) = tool_in_dir(Path::new(&dir), tool) {
            return Ok(path);
        }
    }

    for dir in candidate_dirs(app) {
        if let Some(path) = tool_in_dir(&dir.join("binaries"), tool) {
            return Ok(path);
        }
        if let Some(path) = tool_in_dir(&dir, tool) {
            return Ok(path);
        }
    }

    if which_like(tool).is_some() {
        return Ok(PathBuf::from(tool));
    }

    Err(QuickGifError::ToolNotFound {
        tool: tool.to_string(),
    })
}

fn candidate_dirs(app: &AppHandle) -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(dir) = app.path().resource_dir() {
        dirs.push(dir);
    }
    if let Ok(dir) = env::current_dir() {
        dirs.push(dir.clone());
        dirs.push(dir.join("src-tauri"));
    }
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            dirs.push(parent.to_path_buf());
            dirs.push(parent.join("resources"));
        }
    }

    dirs
}

fn tool_in_dir(dir: &Path, tool: &str) -> Option<PathBuf> {
    let names = tool_names(tool);
    names
        .iter()
        .map(|name| dir.join(name))
        .find(|candidate| candidate.is_file())
}

fn tool_names(tool: &str) -> Vec<String> {
    #[cfg(windows)]
    {
        let mut names = vec![tool.to_string()];
        names.push(format!("{tool}.exe"));
        names
    }
    #[cfg(not(windows))]
    {
        vec![tool.to_string()]
    }
}

fn which_like(tool: &str) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    env::split_paths(&paths)
        .flat_map(|dir| tool_names(tool).into_iter().map(move |name| dir.join(name)))
        .find(|candidate| candidate.is_file())
}
