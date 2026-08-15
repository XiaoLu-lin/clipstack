//! 来源 App 信息：剪贴板变化时读取系统前台 App（bundle id + 本地化名称），
//! 并按 bundle id 解析 64px PNG 图标（返回 data URL）。
//! macOS 实现；其他平台返回 None。

/// 当前前台 App（复制动作的来源）。返回 (bundle_id, 本地化显示名)
#[cfg(target_os = "macos")]
pub fn frontmost_app() -> Option<(String, String)> {
    use objc2::rc::autoreleasepool;
    use objc2_app_kit::NSWorkspace;

    autoreleasepool(|_| {
        let ws = NSWorkspace::sharedWorkspace();
        let app = ws.frontmostApplication()?;
        let bid = app.bundleIdentifier()?.to_string();
        // 排除自己
        if bid == "com.user.clipstack" {
            return None;
        }
        let name = app
            .localizedName()
            .map(|s| s.to_string())
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| bid.clone());
        Some((bid, name))
    })
}

#[cfg(not(target_os = "macos"))]
pub fn frontmost_app() -> Option<(String, String)> {
    None
}

/// bundle id → 64px 图标 data URL（开销较大，调用方需自行缓存）
#[cfg(target_os = "macos")]
pub fn icon_for(bundle_id: &str) -> Option<String> {
    use base64::Engine;
    use objc2::rc::autoreleasepool;
    use objc2_app_kit::{NSBitmapImageFileType, NSBitmapImageRep, NSWorkspace};
    use objc2_foundation::{NSDictionary, NSString};

    autoreleasepool(|_| {
        let ws = NSWorkspace::sharedWorkspace();
        let bid = NSString::from_str(bundle_id);
        let url = ws.URLForApplicationWithBundleIdentifier(&bid)?;
        let path = url.path()?;
        let icon = ws.iconForFile(&path);
        icon.setSize(objc2_foundation::NSSize::new(64.0, 64.0));
        let tiff = icon.TIFFRepresentation()?;
        let rep = NSBitmapImageRep::imageRepWithData(&tiff)?;
        let props = NSDictionary::new();
        let png = unsafe {
            rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &props)
        }?;
        Some(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(png.to_vec())
        ))
    })
}

#[cfg(not(target_os = "macos"))]
pub fn icon_for(_bundle_id: &str) -> Option<String> {
    None
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    #[test]
    fn resolves_builtin_app_icon() {
        let icon = super::icon_for("com.apple.Safari");
        assert!(icon.is_some(), "应能解析 Safari 图标");
        assert!(icon.unwrap().starts_with("data:image/png;base64,"));
    }

    #[test]
    fn unknown_bundle_returns_none() {
        assert!(super::icon_for("com.clipstack.no-such-app").is_none());
    }
}
