use crate::{commands, window, AppState};
use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{Emitter, Manager};

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let show = MenuItemBuilder::with_id("show", "显示剪贴板历史").build(app)?;
    let pause = MenuItemBuilder::with_id("pause", "暂停记录").build(app)?;
    let clear = MenuItemBuilder::with_id("clear", "清空历史…").build(app)?;
    let settings = MenuItemBuilder::with_id("settings", "设置…").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "退出剪贴板工具").build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[
            &show,
            &pause,
            &PredefinedMenuItem::separator(app)?,
            &clear,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ])
        .build()?;

    let pause_item = pause.clone();

    // macOS：单色模板图标，自动适配菜单栏深浅色
    #[cfg(target_os = "macos")]
    let tray_icon = {
        let png = include_bytes!("../icons/tray-icon.png");
        let img = image::load_from_memory(png).expect("tray icon decode");
        let rgba = img.to_rgba8();
        let (w, h) = rgba.dimensions();
        tauri::image::Image::new_owned(rgba.into_raw(), w, h)
    };
    // Windows：模板图标在深色任务栏会隐形，用彩色应用图标
    #[cfg(not(target_os = "macos"))]
    let tray_icon = app.default_window_icon().unwrap().clone();

    let builder = TrayIconBuilder::new()
        .icon(tray_icon)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "show" => window::toggle_main(app),
            "pause" => {
                let state = app.state::<AppState>();
                let now_paused =
                    !state.paused.load(std::sync::atomic::Ordering::Relaxed);
                state
                    .paused
                    .store(now_paused, std::sync::atomic::Ordering::Relaxed);
                let _ = pause_item.set_text(if now_paused { "继续记录" } else { "暂停记录" });
                let _ = app.emit("status://updated", ());
            }
            "clear" => {
                let _ = commands::clear_history_inner(app);
            }
            "settings" => commands::open_settings(app),
            "quit" => app.exit(0),
            _ => {}
        });

    // icon_as_template 是 macOS 专属行为
    #[cfg(target_os = "macos")]
    let builder = builder.icon_as_template(true);

    builder.build(app)?;

    Ok(())
}
