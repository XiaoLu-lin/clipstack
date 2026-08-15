import { useEffect, useState } from "react";
import * as api from "./api";
import { applyTheme } from "./theme";
import { checkForUpdate, downloadInstallRelaunch } from "./updater";
import { prettyHotkey } from "./platform";
import type { SourceApp } from "./types";
import "./styles.css";

const CAPACITY_OPTIONS = [
  { label: "100", value: 100 },
  { label: "500", value: 500 },
  { label: "2000", value: 2000 },
  { label: "无限", value: 0 },
];

const THEME_OPTIONS = [
  { label: "浅色", value: "light" },
  { label: "深色", value: "dark" },
  { label: "跟随系统", value: "system" },
];

type Tab = "general" | "hotkey" | "ignore" | "storage" | "about";

/** KeyboardEvent → Tauri 快捷键字符串；纯修饰键返回 null */
function eventToHotkey(e: KeyboardEvent): string | null {
  const key = e.key;
  if (["Meta", "Control", "Alt", "Shift"].includes(key)) return null;
  const mods: string[] = [];
  if (e.metaKey) mods.push("CommandOrControl");
  if (e.ctrlKey) mods.push("Control");
  if (e.altKey) mods.push("Alt");
  if (e.shiftKey) mods.push("Shift");
  if (mods.length === 0) return null; // 至少需要一个修饰键
  let k: string | null = null;
  if (/^f\d{1,2}$/i.test(key)) k = key.toUpperCase();
  else if (key === " ") k = "Space";
  else if (key === "Enter") k = "Enter";
  else if (key === "Tab") k = "Tab";
  else if (key.startsWith("Arrow")) k = key; // ArrowUp 等
  else if (key.length === 1) k = key.toUpperCase();
  if (!k) return null;
  return [...mods, k].join("+");
}

function Settings() {
  const [tab, setTab] = useState<Tab>("general");
  const [capacity, setCapacity] = useState(500);
  const [autostart, setAutostart] = useState(false);
  const [autoPaste, setAutoPaste] = useState(false);
  const [theme, setTheme] = useState("system");
  const [hotkey, setHotkey] = useState("CommandOrControl+Shift+V");
  const [recording, setRecording] = useState(false);
  const [hotkeyError, setHotkeyError] = useState("");
  const [sourceApps, setSourceApps] = useState<SourceApp[]>([]);
  const [ignored, setIgnored] = useState<string[]>([]);
  const [cleared, setCleared] = useState(false);
  const [updateState, setUpdateState] = useState<"idle" | "checking" | "latest" | "available" | "downloading">("idle");
  const [updateVersion, setUpdateVersion] = useState("");
  const [updatePct, setUpdatePct] = useState(0);

  const refresh = async () => {
    const s = await api.getStatus();
    setCapacity(s.capacity);
    setAutostart(s.autostart);
    setAutoPaste(s.autoPaste);
    setTheme(s.theme);
    setHotkey(s.hotkey);
    applyTheme(s.theme);
  };

  useEffect(() => {
    refresh();
    api.getSourceApps().then(setSourceApps);
    api.getIgnoredApps().then(setIgnored);
    const un = api.onSettingsUpdated(refresh);
    return () => {
      un.then((f) => f());
    };
  }, []);

  // 快捷键录制
  useEffect(() => {
    if (!recording) return;
    setHotkeyError("");
    const onKey = async (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === "Escape") {
        setRecording(false);
        return;
      }
      const combo = eventToHotkey(e);
      if (!combo) return;
      try {
        await api.setHotkey(combo);
        setHotkey(combo);
        setRecording(false);
      } catch (err) {
        setHotkeyError(String(err));
        setRecording(false);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [recording]);

  const handleCheckUpdate = async () => {
    setUpdateState("checking");
    const info = await checkForUpdate();
    if (info) {
      setUpdateVersion(info.version);
      setUpdateState("available");
    } else {
      setUpdateState("latest");
      setTimeout(() => setUpdateState("idle"), 3000);
    }
  };

  const handleDoUpdate = async () => {
    setUpdateState("downloading");
    setUpdatePct(0);
    try {
      await downloadInstallRelaunch((d, total) => {
        setUpdatePct(total > 0 ? Math.round((d / total) * 100) : 0);
      });
      // 成功后自动重启
    } catch {
      setUpdateState("available");
    }
  };

  const toggleIgnore = async (bundle: string, nowRecording: boolean) => {
    // nowRecording=true 表示当前在记录 → 点击后变为忽略
    const toIgnore = nowRecording;
    await api.setIgnoredApp(bundle, toIgnore);
    const list = await api.getIgnoredApps();
    setIgnored(list);
  };

  const rail = (id: Tab, label: string, icon: React.ReactNode) => (
    <div className={`rail-item ${tab === id ? "active" : ""}`} onClick={() => setTab(id)}>
      {icon}
      {label}
    </div>
  );

  const toggle = (on: boolean, onChange: () => void) => (
    <div className={`toggle ${on ? "" : "off"}`} onClick={onChange} />
  );

  return (
    <div className="settings">
      <div className="rail">
        {rail(
          "general",
          "通用",
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h.01a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51h.01a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v.01a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" /></svg>,
        )}
        {rail(
          "hotkey",
          "快捷键",
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8"><rect x="2" y="6" width="20" height="12" rx="2" /><path d="M6 10h.01M10 10h.01M14 10h.01M18 10h.01M6 14h.01M18 14h.01M9 14h6" /></svg>,
        )}
        {rail(
          "ignore",
          "忽略规则",
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8"><circle cx="12" cy="12" r="9" /><path d="M5.5 5.5l13 13" /></svg>,
        )}
        {rail(
          "storage",
          "存储",
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8"><ellipse cx="12" cy="5" rx="8" ry="3" /><path d="M4 5v14c0 1.7 3.6 3 8 3s8-1.3 8-3V5" /><path d="M4 12c0 1.7 3.6 3 8 3s8-1.3 8-3" /></svg>,
        )}
        {rail(
          "about",
          "关于",
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8"><circle cx="12" cy="12" r="9" /><path d="M12 11v5M12 8h.01" /></svg>,
        )}
      </div>

      <div className="content">
        {tab === "general" && (
          <>
            <div className="pref-group">
              <div className="pref-row">
                <div>开机时自动启动<div className="desc">登录后自动在后台运行</div></div>
                {toggle(autostart, async () => {
                  const next = !autostart;
                  setAutostart(next);
                  await api.setAutostart(next);
                })}
              </div>
              <div className="pref-row">
                <div>选择后自动粘贴<div className="desc">开启后 ⏎ 直接粘贴到目标应用（需辅助功能权限）</div></div>
                {toggle(autoPaste, async () => {
                  const next = !autoPaste;
                  setAutoPaste(next);
                  await api.setAutoPaste(next);
                })}
              </div>
              <div className="pref-row">
                <div>外观</div>
                <div className="seg">
                  {THEME_OPTIONS.map((o) => (
                    <span
                      key={o.value}
                      className={theme === o.value ? "on" : ""}
                      onClick={async () => {
                        setTheme(o.value);
                        applyTheme(o.value);
                        await api.setTheme(o.value);
                      }}
                    >
                      {o.label}
                    </span>
                  ))}
                </div>
              </div>
            </div>
          </>
        )}

        {tab === "hotkey" && (
          <>
            <div className="pref-group">
              <div className="pref-row">
                <div>唤起窗口<div className="desc">全局快捷键，随时呼出剪贴板</div></div>
                <button
                  className={`hotkey-btn ${recording ? "recording" : ""}`}
                  onClick={() => setRecording(!recording)}
                >
                  {recording ? "按下新的快捷键…" : prettyHotkey(hotkey)}
                </button>
              </div>
            </div>
            {hotkeyError && <div className="seg-note" style={{ color: "#ff3b30" }}>{hotkeyError}</div>}
            <div className="seg-note">
              点击右侧按钮后按下新组合键（需含 ⌘/⌃/⌥/⇧ 至少一个修饰键），esc 取消。
            </div>
          </>
        )}

        {tab === "ignore" && (
          <>
            <div className="seg-note" style={{ margin: "0 2px 12px" }}>
              关闭「记录」后，对应应用的复制内容将不再进入历史。数据来自历史记录中出现的应用。
            </div>
            {sourceApps.length === 0 ? (
              <div className="seg-note">暂无可管理的应用（先去复制几条内容吧）</div>
            ) : (
              <div className="pref-group">
                {sourceApps.map((app) => {
                  const recording = !ignored.includes(app.bundle);
                  return (
                    <div className="pref-row" key={app.bundle}>
                      <div className="app-cell">
                        {app.icon ? (
                          <img className="app-cell-icon" src={app.icon} alt="" />
                        ) : (
                          <div className="app-cell-icon placeholder" />
                        )}
                        <div>
                          <div>{app.name || app.bundle}</div>
                          <div className="desc">{app.bundle}</div>
                        </div>
                      </div>
                      {toggle(recording, () => toggleIgnore(app.bundle, recording))}
                    </div>
                  );
                })}
              </div>
            )}
          </>
        )}

        {tab === "storage" && (
          <>
            <div className="pref-group">
              <div className="pref-row">
                <div>历史容量</div>
                <div className="seg">
                  {CAPACITY_OPTIONS.map((o) => (
                    <span
                      key={o.value}
                      className={capacity === o.value ? "on" : ""}
                      onClick={async () => {
                        setCapacity(o.value);
                        await api.setCapacity(o.value);
                      }}
                    >
                      {o.label}
                    </span>
                  ))}
                </div>
              </div>
            </div>
            <div className="seg-note">达到容量上限后，将自动删除最早的记录以释放空间。</div>
            <div className="pref-group">
              <div className="pref-row">
                <div>清空历史<div className="desc">删除全部未置顶的记录（置顶内容保留）</div></div>
                <button
                  className="btn-danger"
                  onClick={async () => {
                    await api.clearHistory();
                    setCleared(true);
                    setTimeout(() => setCleared(false), 2000);
                  }}
                >
                  {cleared ? "已清空 ✓" : "清空…"}
                </button>
              </div>
            </div>
          </>
        )}

        {tab === "about" && (
          <div>
            <div className="about-logo">
              <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="1.8"><rect x="5" y="4" width="14" height="17" rx="2.5" /><path d="M9 4.5V3.8A1.8 1.8 0 0 1 10.8 2h2.4A1.8 1.8 0 0 1 15 3.8v.7" /><path d="M9 10h6M9 14h4" strokeLinecap="round" /></svg>
            </div>
            <div className="about-name">ClipStack</div>
            <div className="about-version">版本 0.1.0</div>

            <div className="update-area">
              {updateState === "idle" && (
                <button className="pill update-pill" onClick={handleCheckUpdate}>
                  检查更新
                </button>
              )}
              {updateState === "checking" && <div className="update-text">检查中…</div>}
              {updateState === "latest" && <div className="update-text">已是最新版本 ✓</div>}
              {updateState === "available" && (
                <div className="update-text">
                  发现新版本 <b>v{updateVersion}</b>
                  <button className="pill update-pill" style={{ marginLeft: 10 }} onClick={handleDoUpdate}>
                    立即更新
                  </button>
                </div>
              )}
              {updateState === "downloading" && (
                <div className="update-text">
                  {updatePct > 0 ? `下载中 ${updatePct}%…` : "下载中…"}（完成后自动重启）
                </div>
              )}
            </div>

            <div className="about-version" style={{ marginTop: 16 }}>
              轻量、键盘优先的剪贴板历史工具
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default Settings;
