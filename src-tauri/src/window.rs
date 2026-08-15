use tauri::{App, AppHandle, Emitter, Manager};

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

static LAST_SHOWN_AT: std::sync::Mutex<i64> = std::sync::Mutex::new(0);
/// 窗口是否真正获得过焦点（全屏 Space 焦点回弹会产生假失焦）
static HAD_FOCUS: std::sync::Mutex<bool> = std::sync::Mutex::new(false);

/// 主窗口失焦自动隐藏（剪贴板工具的标准行为），首次启动居中
pub fn setup(app: &App) -> tauri::Result<()> {
    let win = app.get_webview_window("main").unwrap();
    let _ = win.center();
    // 允许窗口出现在任何 Space（含全屏 Space）
    let _ = win.set_visible_on_all_workspaces(true);
    #[cfg(target_os = "macos")]
    {
        convert_to_panel(&win);
        hide_traffic_lights(&win);
    }
    let w = win.clone();
    win.on_window_event(move |event| {
        match event {
            tauri::WindowEvent::Focused(true) => {
                *HAD_FOCUS.lock().unwrap() = true;
            }
            tauri::WindowEvent::Focused(false) => {
                // 唤起后的宽限期内忽略失焦（全屏 Space 切换会补发 Focused(false)）
                let last = *LAST_SHOWN_AT.lock().unwrap();
                if now_millis() - last < 400 {
                    return;
                }
                // 从未获得过焦点的失焦 = 全屏焦点回弹，忽略（窗口保持显示）
                let had = *HAD_FOCUS.lock().unwrap();
                if !had {
                    return;
                }
                *HAD_FOCUS.lock().unwrap() = false;
                let _ = w.hide();
            }
            _ => {}
        }
    });
    Ok(())
}

/// 把 tao 创建的 NSWindow 原地替换成 NSPanel，并设为「非激活面板」。
/// 非激活面板能在不激活本 App 的前提下成为 key window 接收键盘输入，
/// 从而不与全屏 App 抢焦点、不触发焦点回弹——这是 Maccy/Raycast 的底层实现。
#[cfg(target_os = "macos")]
fn convert_to_panel(win: &tauri::WebviewWindow) {
    use objc2::runtime::{AnyClass, AnyObject};
    use objc2::ClassType;
    use objc2_app_kit::{
        NSPanel, NSScreenSaverWindowLevel, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask,
    };
    if let Ok(ptr) = win.ns_window() {
        let ns = ptr as *mut NSWindow;
        if ns.is_null() {
            return;
        }
        unsafe {
            // 换类：NSWindow -> NSPanel
            let panel_class: &AnyClass = NSPanel::class();
            objc2::ffi::object_setClass(ns as *mut AnyObject, panel_class as *const AnyClass);

            let panel: &NSPanel = &*(ns as *mut NSPanel);
            let mut mask = panel.styleMask();
            mask |= NSWindowStyleMask::NonactivatingPanel;
            panel.setStyleMask(mask);
            panel.setLevel(NSScreenSaverWindowLevel);
            let mut behavior = panel.collectionBehavior();
            behavior |= NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary;
            panel.setCollectionBehavior(behavior);
        }
    }
}

/// 隐藏交通灯按钮（保留系统圆角、阴影与标题栏拖动）
#[cfg(target_os = "macos")]
fn hide_traffic_lights(win: &tauri::WebviewWindow) {
    use objc2_app_kit::{NSWindow, NSWindowButton};
    if let Ok(ptr) = win.ns_window() {
        let ns = ptr as *mut NSWindow;
        if ns.is_null() {
            return;
        }
        unsafe {
            let w: &NSWindow = &*ns;
            for btn in [
                NSWindowButton::CloseButton,
                NSWindowButton::MiniaturizeButton,
                NSWindowButton::ZoomButton,
            ] {
                if let Some(b) = w.standardWindowButton(btn) {
                    b.setHidden(true);
                }
            }
        }
    }
}

pub fn toggle_main(app: &AppHandle) {
    let Some(win) = app.get_webview_window("main") else { return };
    let visible = win.is_visible().unwrap_or(false);
    if visible {
        let _ = win.hide();
    } else {
        *LAST_SHOWN_AT.lock().unwrap() = now_millis();
        *HAD_FOCUS.lock().unwrap() = false;
        #[cfg(target_os = "macos")]
        {
            // 全部 AppKit 操作切到主线程，按 移动→显示→激活 顺序执行
            let w = win.clone();
            let _ = app.run_on_main_thread(move || {
                move_to_mouse_screen(&w);
                let _ = w.show();
                order_front(&w);
            });
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = win.center();
            let _ = win.show();
            let _ = win.set_focus();
        }
        let _ = app.emit("window://shown", ());
    }
}

/// 把窗口移动到鼠标所在屏幕并居中（启动器标准策略：跟随当前活跃屏幕，
/// 全屏 Space 场景下尤其重要——win.center() 只会居中窗口原来所属的屏幕）
#[cfg(target_os = "macos")]
fn move_to_mouse_screen(win: &tauri::WebviewWindow) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSEvent, NSScreen, NSWindow};
    use objc2_foundation::NSPoint;

    let Some(mtm) = MainThreadMarker::new() else { return };
    if let Ok(ptr) = win.ns_window() {
        let ns = ptr as *mut NSWindow;
        if ns.is_null() {
            return;
        }
        unsafe {
            let w: &NSWindow = &*ns;
            let mouse = NSEvent::mouseLocation();
            let screens = NSScreen::screens(mtm);
            for screen in screens.iter() {
                // 命中用整屏 frame（含菜单栏），居中用 visibleFrame
                let full = screen.frame();
                let inside = mouse.x >= full.origin.x
                    && mouse.x < full.origin.x + full.size.width
                    && mouse.y >= full.origin.y
                    && mouse.y < full.origin.y + full.size.height;
                if inside {
                    let vis = screen.visibleFrame();
                    let wf = w.frame();
                    w.setFrameOrigin(NSPoint {
                        x: vis.origin.x + (vis.size.width - wf.size.width) / 2.0,
                        y: vis.origin.y + (vis.size.height - wf.size.height) / 2.0,
                    });
                    break;
                }
            }
        }
    }
}

/// 把面板提到最前并设为 key window（不激活本 App）。
/// 非激活面板可在不抢占系统激活状态的情况下接收键盘输入。
/// 必须在主线程调用（toggle_main 已通过 run_on_main_thread 保证）。
#[cfg(target_os = "macos")]
fn order_front(win: &tauri::WebviewWindow) {
    use objc2_app_kit::NSWindow;
    if let Ok(ptr) = win.ns_window() {
        let ns = ptr as *mut NSWindow;
        if ns.is_null() {
            return;
        }
        unsafe {
            let w: &NSWindow = &*ns;
            w.makeKeyAndOrderFront(None);
            w.orderFrontRegardless();
        }
    }
}

pub fn hide_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.hide();
    }
}
