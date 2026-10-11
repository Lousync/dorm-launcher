import { open } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import type { GameStatus } from "./types";

export const STATUS_LABEL: Record<GameStatus, string> = {
  notInstalled: "未安装",
  upToDate: "已是最新",
  updateAvailable: "可更新",
};

export const STATUS_CLASS: Record<GameStatus, string> = {
  notInstalled: "none",
  upToDate: "ok",
  updateAvailable: "new",
};

/** 字节 → 人类可读（55.0 MB）。 */
export function formatSize(bytes?: number | null): string {
  if (!bytes && bytes !== 0) return "—";
  const mb = bytes / (1024 * 1024);
  if (mb >= 1024) return `${(mb / 1024).toFixed(2)} GB`;
  return `${mb.toFixed(1)} MB`;
}

/** ISO 时间 → YYYY-MM-DD。 */
export function formatDate(iso?: string | null): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())}`;
}

export function formatTime(secs?: number | null): string {
  if (!secs) return "—";
  const d = new Date(secs * 1000);
  const p = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${p(d.getMonth() + 1)}-${p(d.getDate())} ${p(d.getHours())}:${p(d.getMinutes())}`;
}

/** 在资源管理器里打开一个目录；成功返回 true。 */
export async function openFolder(path?: string | null): Promise<boolean> {
  if (!path) return false;
  try {
    await openPath(path);
    return true;
  } catch {
    return false;
  }
}

/** 弹系统目录选择框；取消返回 null。 */
export async function pickDirectory(
  defaultPath?: string | null,
): Promise<string | null> {
  const res = await open({
    directory: true,
    multiple: false,
    defaultPath: defaultPath || undefined,
  });
  return typeof res === "string" ? res : null;
}
