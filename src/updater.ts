import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export interface UpdateInfo {
  version: string;
  body?: string;
}

/** 静默检查更新，返回新版本信息；无更新/无网络/无 Release 时返回 null */
export async function checkForUpdate(): Promise<UpdateInfo | null> {
  try {
    const update = await check();
    if (!update) return null;
    return { version: update.version, body: update.body };
  } catch {
    return null;
  }
}

/**
 * 下载并安装最新版本，完成后重启应用。
 * onProgress(downloadedBytes, totalBytes)；total 可能为 0（未知大小）。
 */
export async function downloadInstallRelaunch(
  onProgress?: (downloaded: number, total: number) => void,
): Promise<void> {
  const update = await check();
  if (!update) throw new Error("already up to date");

  let downloaded = 0;
  let total = 0;
  await update.downloadAndInstall((e) => {
    if (e.event === "Started") {
      total = e.data.contentLength ?? 0;
    } else if (e.event === "Progress") {
      downloaded += e.data.chunkLength;
    }
    onProgress?.(downloaded, total);
  });
  await relaunch();
}
