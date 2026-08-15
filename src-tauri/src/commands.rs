use crate::{db, db::ClipItem, paste, window, AppState};
use base64::Engine;
use clipboard_rs::common::RustImage;
use clipboard_rs::{Clipboard, ClipboardContext};
use tauri::{AppHandle, Emitter, Manager};

#[tauri::command]
pub fn list_clips(app: AppHandle, query: Option<String>) -> Result<Vec<ClipItem>, String> {
    let state = app.state::<AppState>();
    let conn = state.db.lock().unwrap();
    let capacity: i64 = db::get_setting(&conn, "capacity")
        .and_then(|v| v.parse().ok())
        .unwrap_or(500);
    let limit = if capacity <= 0 { i64::MAX } else { capacity };
    db::list(&conn, query.as_deref().filter(|q| !q.is_empty()), limit).map_err(|e| e.to_string())
}

/// 把记录写回系统剪贴板。rich=true 时文本/链接会带上 HTML 表示（富文本粘贴）。
fn write_clipboard(app: &AppHandle, id: i64, rich: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    let (item, html) = {
        let conn = state.db.lock().unwrap();
        let item = db::get(&conn, id).map_err(|e| e.to_string())?.ok_or("记录不存在")?;
        let html = if rich {
            db::get_html(&conn, id).ok().flatten().filter(|h| !h.trim().is_empty())
        } else {
            None
        };
        (item, html)
    };

    *state.last_self_write.lock().unwrap() = Some(item.hash.clone());

    let ctx = ClipboardContext::new().map_err(|e| e.to_string())?;
    match item.kind.as_str() {
        "image" => {
            let path = state
                .images_dir
                .join(format!("{}.png", &item.hash[..16.min(item.hash.len())]));
            let img = clipboard_rs::common::RustImageData::from_path(&path.to_string_lossy())
                .map_err(|e| e.to_string())?;
            ctx.set_image(img).map_err(|e| e.to_string())?;
        }
        "file" => {
            let paths: Vec<String> = item
                .meta
                .as_deref()
                .and_then(|m| serde_json::from_str(m).ok())
                .unwrap_or_default();
            if !paths.is_empty() {
                ctx.set_files(paths).map_err(|e| e.to_string())?;
            }
        }
        _ => {
            // 同一内容写入多个表示：纯文本 + （可选）HTML，目标应用按需取
            let mut contents = vec![clipboard_rs::ClipboardContent::Text(item.text.clone())];
            if let Some(h) = html {
                contents.push(clipboard_rs::ClipboardContent::Html(h));
            }
            ctx.set(contents).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// 复制（富文本：文本+HTML）
#[tauri::command]
pub fn copy_clip(app: AppHandle, id: i64) -> Result<(), String> {
    write_clipboard(&app, id, true)
}

/// 复制为纯文本
#[tauri::command]
pub fn copy_clip_plain(app: AppHandle, id: i64) -> Result<(), String> {
    write_clipboard(&app, id, false)
}

/// 粘贴：先隐藏窗口让焦点回到目标应用，写回剪贴板，再模拟按键
#[tauri::command]
pub fn paste_clip(app: AppHandle, id: i64) -> Result<(), String> {
    window::hide_main(&app);
    write_clipboard(&app, id, true)?;
    std::thread::sleep(std::time::Duration::from_millis(150));
    paste::simulate_paste()
}

/// 纯文本粘贴（剥掉格式）
#[tauri::command]
pub fn paste_clip_plain(app: AppHandle, id: i64) -> Result<(), String> {
    window::hide_main(&app);
    write_clipboard(&app, id, false)?;
    std::thread::sleep(std::time::Duration::from_millis(150));
    paste::simulate_paste()
}

#[tauri::command]
pub fn toggle_pin(app: AppHandle, id: i64) -> Result<(), String> {
    let state = app.state::<AppState>();
    let pinned = {
        let conn = state.db.lock().unwrap();
        let item = db::get(&conn, id).map_err(|e| e.to_string())?.ok_or("记录不存在")?;
        db::set_pinned(&conn, id, !item.pinned).map_err(|e| e.to_string())?;
        !item.pinned
    };
    let _ = pinned;
    app.emit("clipboard://updated", ()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_clip(app: AppHandle, id: i64) -> Result<(), String> {
    let state = app.state::<AppState>();
    {
        let conn = state.db.lock().unwrap();
        db::delete(&conn, id).map_err(|e| e.to_string())?;
    }
    app.emit("clipboard://updated", ()).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn clear_history_inner(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let hashes = {
        let conn = state.db.lock().unwrap();
        db::clear_unpinned(&conn).map_err(|e| e.to_string())?
    };
    for h in hashes {
        let _ = std::fs::remove_file(
            state.images_dir.join(format!("{}.png", &h[..16.min(h.len())])),
        );
    }
    app.emit("clipboard://updated", ()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn clear_history(app: AppHandle) -> Result<(), String> {
    clear_history_inner(&app)
}

#[tauri::command]
pub fn set_paused(app: AppHandle, paused: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    state
        .paused
        .store(paused, std::sync::atomic::Ordering::Relaxed);
    Ok(())
}

const DEFAULT_HOTKEY: &str = "CommandOrControl+Shift+V";

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub paused: bool,
    pub capacity: i64,
    pub autostart: bool,
    pub theme: String,
    pub auto_paste: bool,
    pub hotkey: String,
}

fn read_setting(conn: &rusqlite::Connection, key: &str, default: &str) -> String {
    db::get_setting(conn, key).unwrap_or_else(|| default.to_string())
}

#[tauri::command]
pub fn get_status(app: AppHandle) -> Result<Status, String> {
    use tauri_plugin_autostart::ManagerExt;
    let state = app.state::<AppState>();
    let (capacity, theme, auto_paste, hotkey) = {
        let conn = state.db.lock().unwrap();
        (
            read_setting(&conn, "capacity", "500").parse().unwrap_or(500),
            read_setting(&conn, "theme", "system"),
            read_setting(&conn, "auto_paste", "false") == "true",
            read_setting(&conn, "hotkey", DEFAULT_HOTKEY),
        )
    };
    Ok(Status {
        paused: state.paused.load(std::sync::atomic::Ordering::Relaxed),
        capacity,
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        theme,
        auto_paste,
        hotkey,
    })
}

fn persist_and_notify(app: &AppHandle, key: &str, value: &str) -> Result<(), String> {
    let state = app.state::<AppState>();
    {
        let conn = state.db.lock().unwrap();
        db::set_setting(&conn, key, value).map_err(|e| e.to_string())?;
    }
    let _ = app.emit("settings://updated", ());
    Ok(())
}

#[tauri::command]
pub fn set_theme(app: AppHandle, theme: String) -> Result<(), String> {
    persist_and_notify(&app, "theme", &theme)
}

#[tauri::command]
pub fn set_auto_paste(app: AppHandle, enabled: bool) -> Result<(), String> {
    persist_and_notify(&app, "auto_paste", if enabled { "true" } else { "false" })
}

#[tauri::command]
pub fn set_hotkey(app: AppHandle, hotkey: String) -> Result<(), String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
    let new: Shortcut = hotkey.parse().map_err(|_| "无法识别的快捷键组合".to_string())?;
    let state = app.state::<AppState>();
    let old_str = {
        let conn = state.db.lock().unwrap();
        read_setting(&conn, "hotkey", DEFAULT_HOTKEY)
    };
    let gs = app.global_shortcut();
    if let Ok(old) = old_str.parse::<Shortcut>() {
        let _ = gs.unregister(old);
    }
    // 注册失败时回滚旧快捷键，避免快捷键失效
    if let Err(e) = gs.register(new) {
        if let Ok(old) = old_str.parse::<Shortcut>() {
            let _ = gs.register(old);
        }
        return Err(format!("快捷键注册失败（可能被其他应用占用）：{e}"));
    }
    persist_and_notify(&app, "hotkey", &hotkey)
}

// ---------- 忽略规则 ----------

fn read_ignored(conn: &rusqlite::Connection) -> Vec<String> {
    db::get_setting(conn, "ignored_apps")
        .and_then(|v| serde_json::from_str(&v).ok())
        .unwrap_or_default()
}

#[tauri::command]
pub fn get_ignored_apps(app: AppHandle) -> Result<Vec<String>, String> {
    let state = app.state::<AppState>();
    let conn = state.db.lock().unwrap();
    Ok(read_ignored(&conn))
}

#[tauri::command]
pub fn set_ignored_app(app: AppHandle, bundle: String, ignored: bool) -> Result<(), String> {
    let state = app.state::<AppState>();
    let list = {
        let conn = state.db.lock().unwrap();
        let mut list = read_ignored(&conn);
        if ignored {
            if !list.contains(&bundle) {
                list.push(bundle.clone());
            }
        } else {
            list.retain(|b| b != &bundle);
        }
        let json = serde_json::to_string(&list).map_err(|e| e.to_string())?;
        db::set_setting(&conn, "ignored_apps", &json).map_err(|e| e.to_string())?;
        list
    };
    let _ = list;
    let _ = app.emit("settings://updated", ());
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceApp {
    pub bundle: String,
    pub name: String,
    pub icon: Option<String>,
}

/// 历史中出现过的来源 App（供忽略规则页展示）
#[tauri::command]
pub fn get_source_apps(app: AppHandle) -> Result<Vec<SourceApp>, String> {
    let state = app.state::<AppState>();
    let conn = state.db.lock().unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT source_bundle, source, source_icon, MAX(created_at) AS mx
             FROM clips
             WHERE source_bundle IS NOT NULL
             GROUP BY source_bundle
             ORDER BY mx DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(SourceApp {
                bundle: r.get(0)?,
                name: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                icon: r.get(2).ok().flatten(),
            })
        })
        .map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}

#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())?;
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn set_capacity(app: AppHandle, capacity: i64) -> Result<(), String> {
    let state = app.state::<AppState>();
    {
        let conn = state.db.lock().unwrap();
        db::set_setting(&conn, "capacity", &capacity.to_string()).map_err(|e| e.to_string())?;
    }
    app.emit("clipboard://updated", ()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn hide_window(app: AppHandle) -> Result<(), String> {
    window::hide_main(&app);
    Ok(())
}

pub fn open_settings(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.show();
        let _ = win.center();
        let _ = win.set_focus();
        return;
    }
    if let Ok(win) = tauri::WebviewWindowBuilder::new(
        app,
        "settings",
        tauri::WebviewUrl::App("index.html#settings".into()),
    )
    .title("ClipStack 设置")
    .inner_size(660.0, 460.0)
    .resizable(false)
    .center()
    .build()
    {
        let _ = win.set_focus();
    }
}

#[tauri::command]
pub fn open_settings_cmd(app: AppHandle) -> Result<(), String> {
    open_settings(&app);
    Ok(())
}

// 供图片缩略图读取（备用：前端直接拿 data url，此命令保留给大预览）
#[tauri::command]
pub fn get_image_data(app: AppHandle, hash: String) -> Result<String, String> {
    let state = app.state::<AppState>();
    let path = state
        .images_dir
        .join(format!("{}.png", &hash[..16.min(hash.len())]));
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    Ok(format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}
