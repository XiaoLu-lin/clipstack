/** 平台判断（Tauri webview 的 UA 在 Windows 含 "Windows"，macOS 含 "Macintosh"） */
export const IS_MAC = !navigator.userAgent.includes("Windows");

/** 修饰键符号（分平台） */
export const MOD = {
  cmd: IS_MAC ? "⌘" : "Ctrl",
  alt: IS_MAC ? "⌥" : "Alt",
  shift: IS_MAC ? "⇧" : "Shift",
  ctrl: IS_MAC ? "⌃" : "Ctrl",
  enter: IS_MAC ? "⏎" : "Enter",
  backspace: IS_MAC ? "⌫" : "Del",
};

/** 快捷键字符串 → 分平台展示（CommandOrControl+Shift+V → ⌘⇧V / Ctrl+Shift+V） */
export function prettyHotkey(h: string): string {
  const map: Record<string, string> = {
    CommandOrControl: MOD.cmd,
    Command: MOD.cmd,
    Control: MOD.ctrl,
    Alt: MOD.alt,
    Option: MOD.alt,
    Shift: MOD.shift,
    Space: IS_MAC ? "空格" : "Space",
    Enter: MOD.enter,
  };
  const sep = IS_MAC ? " " : "+";
  return h
    .split("+")
    .map((p) => map[p] ?? p.toUpperCase())
    .join(sep);
}
