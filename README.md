# ClipStack

轻量、键盘优先的跨平台剪贴板历史工具。macOS / Windows，一套代码双端。

![platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows-blue)
![tech](https://img.shields.io/badge/Tauri%20v2-Rust%20%2B%20React-orange)

## 功能

- 剪贴板历史监听（文本 / 链接 / 颜色 / 图片 / 文件）
- 全局快捷键唤起（默认 `⇧⌘V` / `Ctrl+Shift+V`，可自定义）
- 键盘优先：↑↓ 选择、⏎ 复制、⌥⏎ 粘贴、⌘数字直达
- 模糊搜索、置顶、来源 App 图标、敏感内容（密码管理器）自动跳过
- 悬浮于全屏应用之上（macOS）、菜单栏 Agent 形态
- 外观（浅色/深色/跟随系统）、自动粘贴、按 App 忽略规则、历史容量管理

## 技术栈

- **外壳**：Tauri v2（Rust 后端 + WebView 前端）
- **前端**：React 19 + TypeScript + Vite
- **后端**：Rust + clipboard-rs + rusqlite（SQLite）+ enigo
- **macOS 原生**：objc2（NSPanel 非激活面板 / 来源 App 图标）

## 开发

```bash
pnpm install
pnpm tauri dev        # 需要 Node 18+、Rust 工具链
```

## 打包

```bash
# macOS
pnpm tauri build

# Windows（在 macOS 上交叉编译，需 brew install llvm + cargo install cargo-xwin）
cd src-tauri && cargo xwin build --release --target x86_64-pc-windows-msvc
```

也可使用 GitHub Actions（`.github/workflows/build.yml`）自动出双端包。

## 文档

- [macOS 全屏悬浮窗口方案](docs/macos-windowing.md)——四层解法（CollectionBehavior / Level / Accessory / NonactivatingPanel）

## License

MIT
