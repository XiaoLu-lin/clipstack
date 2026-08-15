import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ClipItem, SourceApp, Status } from "./types";

export const listClips = (query?: string) =>
  invoke<ClipItem[]>("list_clips", { query: query || null });

export const copyClip = (id: number) => invoke("copy_clip", { id });

export const copyClipPlain = (id: number) => invoke("copy_clip_plain", { id });

export const pasteClip = (id: number) => invoke("paste_clip", { id });

export const pasteClipPlain = (id: number) => invoke("paste_clip_plain", { id });

export const togglePin = (id: number) => invoke("toggle_pin", { id });

export const deleteClip = (id: number) => invoke("delete_clip", { id });

export const clearHistory = () => invoke("clear_history");

export const hideWindow = () => invoke("hide_window");

export const openSettings = () => invoke("open_settings_cmd");

export const getStatus = () => invoke<Status>("get_status");

export const setCapacity = (capacity: number) =>
  invoke("set_capacity", { capacity });

export const setAutostart = (enabled: boolean) =>
  invoke("set_autostart", { enabled });

export const setTheme = (theme: string) => invoke("set_theme", { theme });

export const setAutoPaste = (enabled: boolean) =>
  invoke("set_auto_paste", { enabled });

export const setHotkey = (hotkey: string) => invoke("set_hotkey", { hotkey });

export const getIgnoredApps = () => invoke<string[]>("get_ignored_apps");

export const setIgnoredApp = (bundle: string, ignored: boolean) =>
  invoke("set_ignored_app", { bundle, ignored });

export const getSourceApps = () => invoke<SourceApp[]>("get_source_apps");

export const onClipboardUpdated = (handler: () => void) =>
  listen("clipboard://updated", handler);

export const onStatusUpdated = (handler: () => void) =>
  listen("status://updated", handler);

export const onWindowShown = (handler: () => void) =>
  listen("window://shown", handler);

export const onSettingsUpdated = (handler: () => void) =>
  listen("settings://updated", handler);
