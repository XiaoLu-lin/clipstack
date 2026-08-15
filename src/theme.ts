/** 应用主题设置到 <html data-theme>，light/dark 强制覆盖，system 跟随系统 */
export function applyTheme(theme: string) {
  const root = document.documentElement;
  if (theme === "light" || theme === "dark") {
    root.dataset.theme = theme;
  } else {
    delete root.dataset.theme;
  }
}
