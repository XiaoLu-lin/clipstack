# ClipStack 窗口方案（macOS 全屏悬浮的实现原理）

本文档记录 ClipStack 主窗口如何在 macOS 上实现"悬浮于全屏应用之上、不抢占菜单栏、跟随鼠标屏幕"的完整方案。这是反复调试后的结论，供后续维护参考。

## 问题背景

macOS 的全屏应用运行在一个**独立的 Space（空间）**，且有严格的窗口准入规则。普通窗口默认：
- 只属于"桌面"这个 Space，无法进入全屏 Space
- 唤起时需要激活 App，会和全屏 App 抢焦点，被系统回弹后窗口被隐藏

## 完整解法（四层缺一不可）

在 `src/window.rs` 的 `convert_to_panel` / `setup` 中实现：

### 1. CollectionBehavior —— 窗口"有资格"进入全屏空间

```rust
behavior |= CanJoinAllSpaces      // 可出现在任何 Space
         | FullScreenAuxiliary;   // 可作为辅助窗浮在全屏 App 上
```

### 2. Level —— 层级够高

```rust
panel.setLevel(NSScreenSaverWindowLevel);  // 1000 级
```

全屏 Space 只绘制高层级窗口。普通 floating 级（3）会被挡住。与 uTools（Electron 的 `'screen-saver'` 档）一致。

### 3. Accessory 策略 —— 后台代理应用

```rust
// lib.rs setup
app.set_activation_policy(ActivationPolicy::Accessory);
// src-tauri/Info.plist: LSUIElement = true
```

效果：无 Dock 图标、唤起时不抢占菜单栏（菜单栏保持显示当前 App）、不出现在 ⌘Tab。

### 4. 【关键】NonactivatingPanel —— 非激活面板

这是解决"焦点回弹"的决定性一步。普通 NSWindow 唤起时必须激活 App 才能获得键盘焦点，从而和全屏 App 抢焦点、被系统回弹。把窗口原地换成 NSPanel 并加非激活样式位：

```rust
// 把 tao 创建的 NSWindow 换类成 NSPanel
object_setClass(ns_window, NSPanel::class());
// 加非激活样式位
mask |= NSWindowStyleMask::NonactivatingPanel;
panel.setStyleMask(mask);
```

非激活面板能**在不激活本 App 的前提下成为 key window 接收键盘输入**，不与全屏 App 发生焦点争夺。这正是 Maccy / Raycast / uTools 的底层实现。

## 两个辅助细节

### 主线程

全局快捷键的回调不在主线程，而所有 AppKit 调用必须在主线程执行。`toggle_main` 里通过 `app.run_on_main_thread(...)` 把"移动→显示→置前"三步切到主线程，按顺序执行。

### 多屏定位

`win.center()` 只会居中窗口原来所属的屏幕。唤起时改为"跟随鼠标所在屏幕"（`move_to_mouse_screen`）：取鼠标坐标 → 命中用整屏 `frame()` 判断（含菜单栏区域）→ 居中用 `visibleFrame()`。全屏在哪块屏、面板就出在哪块屏。

## 唤起流程（toggle_main）

```
快捷键回调（后台线程）
  → run_on_main_thread:
      move_to_mouse_screen()   // 移到鼠标屏幕并居中
      win.show()               // Tauri 显示 webview
      order_front()            // makeKeyAndOrderFront + orderFrontRegardless（面板置前，不激活 App）
  → 前端收到 window://shown 事件，清空搜索、聚焦输入框
```

## 失焦自动隐藏

面板失焦（用户点击别处 / 按 esc）→ 自动隐藏。为防全屏 Space 切换时的假失焦误触发，加了两个保护：
- 唤起后 400ms 宽限期内忽略失焦
- 只有"真正获得过焦点"后的失焦才隐藏（`HAD_FOCUS` 追踪）

## 一句话总结

> 资格（CollectionBehavior）+ 层级（ScreenSaver Level）+ 后台代理（Accessory）+ 非激活面板（NSPanel），四层叠加，让窗口能像 Spotlight 一样悬浮在全屏应用之上。其中非激活面板是解决焦点回弹的关键。
