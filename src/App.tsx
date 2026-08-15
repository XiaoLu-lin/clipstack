import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import * as api from "./api";
import type { ClipItem } from "./types";
import { applyTheme } from "./theme";
import { groupLabel, relativeTime, KIND_LABEL } from "./utils";
import "./styles.css";

/** 类型图标（后续接入真实来源 App 图标后替换） */
function TypeIcon({ item }: { item: ClipItem }) {
  const cls = `app-icon icon-${item.kind}`;
  const style =
    item.kind === "color" ? ({ "--swatch": item.text } as React.CSSProperties) : undefined;
  const svg = (() => {
    switch (item.kind) {
      case "link":
        return (
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="2">
            <circle cx="12" cy="12" r="9" />
            <path d="M15.5 8.5l-2 5-5 2 2-5z" fill="#fff" stroke="none" />
          </svg>
        );
      case "image":
        return (
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="1.8">
            <rect x="3" y="4" width="18" height="16" rx="2.5" />
            <circle cx="9" cy="10" r="1.8" />
            <path d="M4 18l5-5 3.5 3.5L16 13l4 4" />
          </svg>
        );
      case "file":
        return (
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="1.8">
            <path d="M7 9v2M17 9v2M6 15c1.5 2 3.6 3 6 3s4.5-1 6-3" />
          </svg>
        );
      case "color":
        return null;
      default:
        return (
          <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="1.8">
            <rect x="5" y="3" width="14" height="18" rx="2" />
            <path d="M9 8h6M9 12h6M9 16h3" strokeLinecap="round" />
          </svg>
        );
    }
  })();
  return (
    <div className={cls} style={style}>
      {svg}
    </div>
  );
}

interface Section {
  label: string;
  items: { item: ClipItem; index: number }[];
}

function buildSections(clips: ClipItem[]): Section[] {
  const pinned: Section = { label: "已置顶", items: [] };
  const groups = new Map<string, Section>();
  clips.forEach((item, index) => {
    if (item.pinned) {
      pinned.items.push({ item, index });
    } else {
      const label = groupLabel(item.createdAt);
      if (!groups.has(label)) groups.set(label, { label, items: [] });
      groups.get(label)!.items.push({ item, index });
    }
  });
  const order = ["今天", "昨天", "更早"];
  const rest = order.filter((l) => groups.has(l)).map((l) => groups.get(l)!);
  return [...(pinned.items.length ? [pinned] : []), ...rest];
}

function App() {
  const [clips, setClips] = useState<ClipItem[]>([]);
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const [paused, setPaused] = useState(false);
  const [autoPaste, setAutoPaste] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const queryRef = useRef(query);
  queryRef.current = query;
  const autoPasteRef = useRef(false);
  autoPasteRef.current = autoPaste;

  const load = useCallback(async (q?: string) => {
    const kw = q ?? queryRef.current;
    setClips(await api.listClips(kw));
    setSelected(0);
  }, []);

  useEffect(() => {
    load();
    const refreshStatus = () =>
      api.getStatus().then((s) => {
        setPaused(s.paused);
        setAutoPaste(s.autoPaste);
        applyTheme(s.theme);
      });
    refreshStatus();
    const unlisten = api.onClipboardUpdated(() => load());
    const unlistenStatus = api.onStatusUpdated(refreshStatus);
    const unlistenSettings = api.onSettingsUpdated(refreshStatus);
    // 窗口每次唤起：清空搜索、回到最新一条、聚焦输入框
    const unlistenShown = api.onWindowShown(() => {
      setQuery("");
      setSelected(0);
      load("");
      setTimeout(() => inputRef.current?.focus(), 50);
    });
    return () => {
      unlisten.then((f) => f());
      unlistenStatus.then((f) => f());
      unlistenSettings.then((f) => f());
      unlistenShown.then((f) => f());
    };
  }, [load]);

  // 搜索防抖
  useEffect(() => {
    const t = setTimeout(() => load(query), 150);
    return () => clearTimeout(t);
  }, [query, load]);

  // 窗口获得焦点时重新聚焦搜索框
  useEffect(() => {
    const onFocus = () => inputRef.current?.focus();
    window.addEventListener("focus", onFocus);
    inputRef.current?.focus();
    return () => window.removeEventListener("focus", onFocus);
  }, []);

  const selectedItem = clips[selected];

  const doCopy = useCallback(
    async (index: number) => {
      const item = clips[index];
      if (!item) return;
      await api.copyClip(item.id);
      await api.hideWindow();
    },
    [clips],
  );

  const doPaste = useCallback(
    async (index: number) => {
      const item = clips[index];
      if (!item) return;
      await api.pasteClip(item.id);
    },
    [clips],
  );

  // 键盘交互
  useEffect(() => {
    const onKey = async (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelected((s) => Math.min(s + 1, clips.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelected((s) => Math.max(s - 1, 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        if (e.altKey) await doPaste(selected);
        else if (autoPasteRef.current) await doPaste(selected); // 自动粘贴：⏎ 直接粘贴
        else await doCopy(selected);
      } else if (mod && /^[1-9]$/.test(e.key)) {
        e.preventDefault();
        await doCopy(parseInt(e.key, 10) - 1);
      } else if (e.altKey && (e.key === "p" || e.key === "P" || e.key === "π")) {
        e.preventDefault();
        if (selectedItem) await api.togglePin(selectedItem.id);
      } else if (e.altKey && e.key === "Backspace") {
        e.preventDefault();
        if (selectedItem) await api.deleteClip(selectedItem.id);
      } else if (e.key === "Escape") {
        if (query) setQuery("");
        else await api.hideWindow();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [clips, selected, query, selectedItem, doCopy, doPaste]);

  // 选中项滚动到可见区域
  const listRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    listRef.current
      ?.querySelector(`[data-index="${selected}"]`)
      ?.scrollIntoView({ block: "nearest" });
  }, [selected]);

  const sections = useMemo(() => buildSections(clips), [clips]);

  const metaOf = (item: ClipItem) => {
    const parts = [KIND_LABEL[item.kind] ?? item.kind];
    if (item.source) parts.push(item.source);
    parts.push(relativeTime(item.createdAt));
    return parts.join(" · ");
  };

  return (
    <div className="app">
      <div className="titlebar">
        <div className="search">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4">
            <circle cx="11" cy="11" r="7" />
            <path d="M20 20l-3.5-3.5" />
          </svg>
          <input
            ref={inputRef}
            placeholder="搜索剪贴板历史…"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <span className="esc-hint">esc</span>
        </div>
      </div>

      {clips.length === 0 ? (
        <div className="empty-body">
          <svg width="52" height="52" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.4">
            <rect x="5" y="4" width="14" height="17" rx="2.5" />
            <path d="M9 4.5V3.8A1.8 1.8 0 0 1 10.8 2h2.4A1.8 1.8 0 0 1 15 3.8v.7" />
            <path d="M9 10h6M9 14h4" strokeLinecap="round" />
          </svg>
          <div className="empty-title">{query ? "没有匹配的记录" : "还没有剪贴记录"}</div>
          <div className="empty-desc">
            {query ? "换个关键词试试" : "复制文本、图片或文件后会显示在这里"}
          </div>
        </div>
      ) : (
        <div className="list" ref={listRef}>
          {sections.map((sec) => (
            <div key={sec.label}>
              <div className="group-label">{sec.label}</div>
              {sec.items.map(({ item, index }) => (
                <div
                  key={item.id}
                  data-index={index}
                  className={`card ${index === selected ? "selected" : ""}`}
                  onClick={() => setSelected(index)}
                  onDoubleClick={() => doCopy(index)}
                >
                  {item.sourceIcon ? (
                    <img className="app-icon app-icon-img" src={item.sourceIcon} alt="" />
                  ) : (
                    <TypeIcon item={item} />
                  )}
                  <div className="card-body">
                    <div className="card-title">
                      {item.kind === "color" || item.kind === "text" ? (
                        <span className={item.text.length > 60 ? "" : "mono"}>{item.text}</span>
                      ) : (
                        item.text
                      )}
                    </div>
                    <div className="meta">{metaOf(item)}</div>
                  </div>
                  {item.kind === "image" && item.thumb && (
                    <img className="thumb" src={item.thumb} alt="" />
                  )}
                  {index < 9 && <span className="kbd">⌘{index + 1}</span>}
                </div>
              ))}
            </div>
          ))}
        </div>
      )}

      <div className="hint-bar">
        <span><b>⏎</b> 复制</span><span className="sep">／</span>
        <span><b>⌥⏎</b> 粘贴</span><span className="sep">／</span>
        <span><b>⌥⇧⏎</b> 纯文本粘贴</span><span className="sep">／</span>
        <span><b>⌥P</b> 置顶</span><span className="sep">／</span>
        <span><b>⌥⌫</b> 删除</span>
      </div>

      <div className="footer">
        <span>
          共 {clips.length} 条记录
          {paused && <span className="paused-badge">已暂停记录</span>}
        </span>
        <div className="pill-btns">
          <button className="pill" onClick={() => api.clearHistory()}>
            清空
          </button>
          <button className="pill" onClick={() => api.openSettings()}>
            设置
          </button>
        </div>
      </div>
    </div>
  );
}

export default App;
