mod appicon;
mod clipboard;
mod commands;
mod db;
mod paste;
mod tray;
mod window;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Mutex;
use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub paused: AtomicBool,
    /// 我们自己写回剪贴板的内容 hash，用于防止监听回环
    pub last_self_write: Mutex<Option<String>>,
    pub images_dir: PathBuf,
    /// bundle id → 图标 data URL 缓存
    pub icon_cache: Mutex<HashMap<String, Option<String>>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        window::toggle_main(app);
                    }
                })
                .build(),
        )
        .setup(|app| {
            // Agent 应用：无 Dock 图标、不抢占菜单栏（uTools/Maccy 同款策略）
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let images_dir = data_dir.join("images");
            std::fs::create_dir_all(&images_dir)?;

            let conn = db::init(&data_dir.join("clipstack.db"))?;
            app.manage(AppState {
                db: Mutex::new(conn),
                paused: AtomicBool::new(false),
                last_self_write: Mutex::new(None),
                images_dir,
                icon_cache: Mutex::new(HashMap::new()),
            });

            tray::setup(app)?;
            window::setup(app)?;
            clipboard::start_watcher(app.handle().clone());

            // 从设置读取快捷键（默认 ⇧⌘V）
            let hotkey_str = {
                let state = app.state::<AppState>();
                let conn = state.db.lock().unwrap();
                db::get_setting(&conn, "hotkey")
                    .unwrap_or_else(|| "CommandOrControl+Shift+V".to_string())
            };
            let shortcut: tauri_plugin_global_shortcut::Shortcut = hotkey_str
                .parse()
                .unwrap_or_else(|_| "CommandOrControl+Shift+V".parse().expect("invalid shortcut"));
            app.global_shortcut().register(shortcut)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_clips,
            commands::copy_clip,
            commands::paste_clip,
            commands::toggle_pin,
            commands::delete_clip,
            commands::clear_history,
            commands::set_paused,
            commands::get_status,
            commands::set_capacity,
            commands::hide_window,
            commands::open_settings_cmd,
            commands::get_image_data,
            commands::set_autostart,
            commands::set_theme,
            commands::set_auto_paste,
            commands::set_hotkey,
            commands::get_ignored_apps,
            commands::set_ignored_app,
            commands::get_source_apps,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
