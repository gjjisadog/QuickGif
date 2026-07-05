mod error_handler;
mod ffmpeg_locator;
mod frame_indexer;
mod gif_exporter;
mod image_exporter;
mod mp4_exporter;
mod settings;
mod temp_manager;
mod thumbnail_generator;
mod video_probe;
mod webp_exporter;

use frame_indexer::{build_frame_index, FramePoint};
use gif_exporter::{export_gif, ExportGifRequest, ExportGifResult};
use image_exporter::{export_frame_image, ExportFrameImageRequest, ExportFrameImageResult};
use mp4_exporter::{export_mp4, ExportMp4Request, ExportMp4Result};
use settings::{
    load_settings, set_default_export_dir, set_language, set_shortcuts, AppSettings,
    KeyboardShortcuts,
};
use std::path::PathBuf;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem, Submenu},
    Emitter,
};
use thumbnail_generator::{generate_detail_thumbnails, generate_thumbnails, ThumbnailResult};
use video_probe::{probe_video, VideoInfo};
use webp_exporter::{export_webp, ExportWebpRequest, ExportWebpResult};

const MENU_LANGUAGE_ZH: &str = "quickgif-language-zh";
const MENU_LANGUAGE_EN: &str = "quickgif-language-en";
const MENU_SET_EXPORT_DIR: &str = "quickgif-set-export-dir";
const MENU_USE_SOURCE_DIR: &str = "quickgif-use-source-dir";
const MENU_EXPORT_FORMAT_MP4: &str = "quickgif-export-format-mp4";
const MENU_EXPORT_FORMAT_WEBP: &str = "quickgif-export-format-webp";
const MENU_EXPORT_FORMAT_GIF: &str = "quickgif-export-format-gif";
const MENU_KEYBOARD_SHORTCUTS: &str = "quickgif-keyboard-shortcuts";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .menu(build_app_menu)
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            if matches!(
                id,
                MENU_LANGUAGE_ZH
                    | MENU_LANGUAGE_EN
                    | MENU_SET_EXPORT_DIR
                    | MENU_USE_SOURCE_DIR
                    | MENU_EXPORT_FORMAT_MP4
                    | MENU_EXPORT_FORMAT_WEBP
                    | MENU_EXPORT_FORMAT_GIF
                    | MENU_KEYBOARD_SHORTCUTS
            ) {
                let _ = app.emit("quickgif-menu", id);
            }
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|_app| {
            temp_manager::cleanup_stale_temp_dirs();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            probe_video_command,
            generate_thumbnails_command,
            generate_detail_thumbnails_command,
            build_frame_index_command,
            export_gif_command,
            export_mp4_command,
            export_webp_command,
            export_frame_image_command,
            get_settings_command,
            set_language_command,
            set_default_export_dir_command,
            set_shortcuts_command
        ])
        .run(tauri::generate_context!())
        .expect("failed to run QuickGif");
}

fn build_app_menu(app: &tauri::AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let settings = load_settings(app).unwrap_or_default();
    let labels = MenuLabels::for_language(&settings.language);

    let language_zh = MenuItem::with_id(
        app,
        MENU_LANGUAGE_ZH,
        labels.language_zh,
        true,
        None::<&str>,
    )?;
    let language_en = MenuItem::with_id(
        app,
        MENU_LANGUAGE_EN,
        labels.language_en,
        true,
        None::<&str>,
    )?;
    let set_export_dir = MenuItem::with_id(
        app,
        MENU_SET_EXPORT_DIR,
        labels.set_export_dir,
        true,
        None::<&str>,
    )?;
    let use_source_dir = MenuItem::with_id(
        app,
        MENU_USE_SOURCE_DIR,
        labels.use_source_dir,
        true,
        None::<&str>,
    )?;
    let export_format_mp4 = MenuItem::with_id(
        app,
        MENU_EXPORT_FORMAT_MP4,
        labels.export_format_mp4,
        true,
        None::<&str>,
    )?;
    let export_format_webp =
        MenuItem::with_id(app, MENU_EXPORT_FORMAT_WEBP, "WebP", true, None::<&str>)?;
    let export_format_gif =
        MenuItem::with_id(app, MENU_EXPORT_FORMAT_GIF, "GIF", true, None::<&str>)?;
    let keyboard_shortcuts = MenuItem::with_id(
        app,
        MENU_KEYBOARD_SHORTCUTS,
        labels.keyboard_shortcuts,
        true,
        None::<&str>,
    )?;
    let language_menu = Submenu::with_items(
        app,
        labels.language_menu,
        true,
        &[&language_zh, &language_en],
    )?;
    let export_format_menu = Submenu::with_items(
        app,
        labels.export_format_menu,
        true,
        &[&export_format_mp4, &export_format_webp, &export_format_gif],
    )?;
    let export_separator = PredefinedMenuItem::separator(app)?;
    let export_menu = Submenu::with_items(
        app,
        labels.export_menu,
        true,
        &[
            &export_format_menu,
            &export_separator,
            &set_export_dir,
            &use_source_dir,
        ],
    )?;
    let settings_menu =
        Submenu::with_items(app, labels.settings_menu, true, &[&keyboard_shortcuts])?;

    Menu::with_items(app, &[&language_menu, &export_menu, &settings_menu])
}

struct MenuLabels {
    language_menu: &'static str,
    language_zh: &'static str,
    language_en: &'static str,
    export_menu: &'static str,
    export_format_menu: &'static str,
    export_format_mp4: &'static str,
    set_export_dir: &'static str,
    use_source_dir: &'static str,
    settings_menu: &'static str,
    keyboard_shortcuts: &'static str,
}

impl MenuLabels {
    fn for_language(language: &str) -> Self {
        if language == "en" {
            return Self {
                language_menu: "Language",
                language_zh: "简体中文",
                language_en: "English",
                export_menu: "Export",
                export_format_menu: "Export Format",
                export_format_mp4: "MP4 (Default)",
                set_export_dir: "Set Default Export Folder...",
                use_source_dir: "Use Source Video Folder",
                settings_menu: "Settings",
                keyboard_shortcuts: "Keyboard Shortcuts...",
            };
        }

        Self {
            language_menu: "语言",
            language_zh: "简体中文",
            language_en: "English",
            export_menu: "导出",
            export_format_menu: "导出格式",
            export_format_mp4: "MP4（默认）",
            set_export_dir: "设置默认导出文件夹...",
            use_source_dir: "使用原视频目录",
            settings_menu: "设置",
            keyboard_shortcuts: "快捷键...",
        }
    }
}

#[tauri::command]
fn probe_video_command(app: tauri::AppHandle, path: String) -> Result<VideoInfo, String> {
    probe_video(&app, path).map_err(|error| error.to_string())
}

#[tauri::command]
async fn generate_thumbnails_command(
    app: tauri::AppHandle,
    path: String,
    duration: f64,
) -> Result<ThumbnailResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = PathBuf::from(path);
        generate_thumbnails(&app, &path, duration)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn generate_detail_thumbnails_command(
    app: tauri::AppHandle,
    path: String,
    start_time: f64,
    duration: f64,
) -> Result<ThumbnailResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = PathBuf::from(path);
        generate_detail_thumbnails(&app, &path, start_time, duration)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn build_frame_index_command(
    app: tauri::AppHandle,
    path: String,
) -> Result<Vec<FramePoint>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = PathBuf::from(path);
        build_frame_index(&app, &path)
    })
    .await
    .map_err(|error| error.to_string())?
    .map_err(|error| error.to_string())
}

#[tauri::command]
async fn export_gif_command(
    app: tauri::AppHandle,
    request: ExportGifRequest,
) -> Result<ExportGifResult, String> {
    tauri::async_runtime::spawn_blocking(move || export_gif(&app, request))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn export_mp4_command(
    app: tauri::AppHandle,
    request: ExportMp4Request,
) -> Result<ExportMp4Result, String> {
    tauri::async_runtime::spawn_blocking(move || export_mp4(&app, request))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn export_webp_command(
    app: tauri::AppHandle,
    request: ExportWebpRequest,
) -> Result<ExportWebpResult, String> {
    tauri::async_runtime::spawn_blocking(move || export_webp(&app, request))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn export_frame_image_command(
    app: tauri::AppHandle,
    request: ExportFrameImageRequest,
) -> Result<ExportFrameImageResult, String> {
    tauri::async_runtime::spawn_blocking(move || export_frame_image(&app, request))
        .await
        .map_err(|error| error.to_string())?
        .map_err(|error| error.to_string())
}

#[tauri::command]
fn get_settings_command(app: tauri::AppHandle) -> Result<AppSettings, String> {
    load_settings(&app).map_err(|error| error.to_string())
}

#[tauri::command]
fn set_language_command(app: tauri::AppHandle, language: String) -> Result<AppSettings, String> {
    let settings = set_language(&app, language).map_err(|error| error.to_string())?;
    let menu = build_app_menu(&app).map_err(|error| error.to_string())?;
    app.set_menu(menu).map_err(|error| error.to_string())?;
    Ok(settings)
}

#[tauri::command]
fn set_default_export_dir_command(
    app: tauri::AppHandle,
    default_export_dir: Option<String>,
) -> Result<AppSettings, String> {
    set_default_export_dir(&app, default_export_dir).map_err(|error| error.to_string())
}

#[tauri::command]
fn set_shortcuts_command(
    app: tauri::AppHandle,
    shortcuts: KeyboardShortcuts,
) -> Result<AppSettings, String> {
    set_shortcuts(&app, shortcuts).map_err(|error| error.to_string())
}
