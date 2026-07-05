use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::error_handler::{QuickGifError, QuickGifResult};

pub const LANGUAGE_ZH_CN: &str = "zh-CN";
pub const LANGUAGE_EN: &str = "en";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub language: String,
    pub default_export_dir: Option<String>,
    #[serde(default)]
    pub shortcuts: KeyboardShortcuts,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardShortcuts {
    pub play_pause: ShortcutBinding,
    pub previous_frame: ShortcutBinding,
    pub next_frame: ShortcutBinding,
    pub set_start_frame: ShortcutBinding,
    pub set_end_frame: ShortcutBinding,
    pub export_clip: ShortcutBinding,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutBinding {
    pub key: String,
    pub code: String,
    pub ctrl_key: bool,
    pub meta_key: bool,
    pub alt_key: bool,
    pub shift_key: bool,
    pub label: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            language: LANGUAGE_ZH_CN.to_string(),
            default_export_dir: None,
            shortcuts: KeyboardShortcuts::default(),
        }
    }
}

impl Default for KeyboardShortcuts {
    fn default() -> Self {
        Self {
            play_pause: ShortcutBinding::new(" ", "Space", false, false, false, false, "Space"),
            previous_frame: ShortcutBinding::new(
                "ArrowLeft",
                "ArrowLeft",
                false,
                false,
                false,
                false,
                "Left",
            ),
            next_frame: ShortcutBinding::new(
                "ArrowRight",
                "ArrowRight",
                false,
                false,
                false,
                false,
                "Right",
            ),
            set_start_frame: ShortcutBinding::new("i", "KeyI", false, false, false, false, "I"),
            set_end_frame: ShortcutBinding::new("o", "KeyO", false, false, false, false, "O"),
            export_clip: ShortcutBinding::new(
                "e",
                "KeyE",
                true,
                true,
                false,
                false,
                "Ctrl/Cmd + E",
            ),
        }
    }
}

impl ShortcutBinding {
    fn new(
        key: &str,
        code: &str,
        ctrl_key: bool,
        meta_key: bool,
        alt_key: bool,
        shift_key: bool,
        label: &str,
    ) -> Self {
        Self {
            key: key.to_string(),
            code: code.to_string(),
            ctrl_key,
            meta_key,
            alt_key,
            shift_key,
            label: label.to_string(),
        }
    }
}

pub fn load_settings(app: &AppHandle) -> QuickGifResult<AppSettings> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let bytes = fs::read(path)?;
    let mut settings = serde_json::from_slice::<AppSettings>(&bytes).unwrap_or_default();
    settings.language = normalize_language(&settings.language).to_string();
    if settings
        .default_export_dir
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        settings.default_export_dir = None;
    }
    Ok(settings)
}

pub fn save_settings(app: &AppHandle, settings: &AppSettings) -> QuickGifResult<()> {
    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut normalized = settings.clone();
    normalized.language = normalize_language(&normalized.language).to_string();
    if normalized
        .default_export_dir
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        normalized.default_export_dir = None;
    }
    fs::write(path, serde_json::to_vec_pretty(&normalized)?)?;
    Ok(())
}

pub fn set_language(app: &AppHandle, language: String) -> QuickGifResult<AppSettings> {
    let mut settings = load_settings(app)?;
    settings.language = normalize_language(&language).to_string();
    save_settings(app, &settings)?;
    Ok(settings)
}

pub fn set_default_export_dir(
    app: &AppHandle,
    default_export_dir: Option<String>,
) -> QuickGifResult<AppSettings> {
    let mut settings = load_settings(app)?;
    settings.default_export_dir = default_export_dir.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    });
    save_settings(app, &settings)?;
    Ok(settings)
}

pub fn set_shortcuts(app: &AppHandle, shortcuts: KeyboardShortcuts) -> QuickGifResult<AppSettings> {
    let mut settings = load_settings(app)?;
    settings.shortcuts = shortcuts;
    save_settings(app, &settings)?;
    Ok(settings)
}

pub fn resolve_output_dir(input_path: &Path, settings: &AppSettings) -> PathBuf {
    settings
        .default_export_dir
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            input_path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf()
        })
}

pub fn normalize_language(language: &str) -> &'static str {
    match language {
        LANGUAGE_EN | "en-US" | "en-GB" => LANGUAGE_EN,
        LANGUAGE_ZH_CN | "zh" | "zh-Hans" => LANGUAGE_ZH_CN,
        _ => LANGUAGE_ZH_CN,
    }
}

fn settings_path(app: &AppHandle) -> QuickGifResult<PathBuf> {
    app.path()
        .app_config_dir()
        .map(|path| path.join("settings.json"))
        .map_err(|error| QuickGifError::Io(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn default_settings_use_chinese_and_source_folder() {
        let settings = AppSettings::default();

        assert_eq!(settings.language, "zh-CN");
        assert_eq!(settings.default_export_dir, None);
        assert_eq!(settings.shortcuts.play_pause.label, "Space");
        assert_eq!(settings.shortcuts.export_clip.label, "Ctrl/Cmd + E");
    }

    #[test]
    fn fills_default_shortcuts_when_loading_legacy_settings_json() {
        let settings = serde_json::from_str::<AppSettings>(
            r#"{"language":"en","defaultExportDir":"/tmp/export"}"#,
        )
        .expect("legacy settings should deserialize");

        assert_eq!(settings.language, "en");
        assert_eq!(settings.default_export_dir, Some("/tmp/export".to_string()));
        assert_eq!(settings.shortcuts.previous_frame.code, "ArrowLeft");
        assert_eq!(settings.shortcuts.set_end_frame.label, "O");
    }

    #[test]
    fn normalizes_supported_language_values() {
        assert_eq!(normalize_language("en"), "en");
        assert_eq!(normalize_language("en-US"), "en");
        assert_eq!(normalize_language("zh"), "zh-CN");
        assert_eq!(normalize_language("zh-CN"), "zh-CN");
        assert_eq!(normalize_language("fr"), "zh-CN");
    }

    #[test]
    fn uses_default_export_directory_when_present() {
        let input = Path::new("/tmp/source/video.mp4");
        let settings = AppSettings {
            language: "en".to_string(),
            default_export_dir: Some("/tmp/export".to_string()),
            shortcuts: KeyboardShortcuts::default(),
        };

        assert_eq!(
            resolve_output_dir(input, &settings),
            PathBuf::from("/tmp/export")
        );
    }

    #[test]
    fn falls_back_to_source_directory_without_default_export_directory() {
        let input = Path::new("/tmp/source/video.mp4");
        let settings = AppSettings::default();

        assert_eq!(
            resolve_output_dir(input, &settings),
            PathBuf::from("/tmp/source")
        );
    }
}
