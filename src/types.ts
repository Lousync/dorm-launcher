/**
 * 与 Rust 后端 `model.rs` 一一对应的类型。
 * 字段名走 camelCase（serde 侧已按此序列化）。
 */

export interface AppInfo {
  name: string;
  version: string;
}

export type GameStatus = "notInstalled" | "upToDate" | "updateAvailable";

export interface GameVersionView {
  version: string;
  url: string;
  sha256: string;
  size?: number | null;
  publishedAt?: string | null;
  notes?: string | null;
}

export interface GameView {
  id: string;
  name: string;
  summary?: string | null;
  description?: string | null;
  cover?: string | null;
  screenshots: string[];
  executable: string;
  args: string[];
  userDataNote?: string | null;
  latestVersion?: string | null;
  installedVersion?: string | null;
  installedVersions: string[];
  versions: GameVersionView[];
  status: GameStatus;
  installPath?: string | null;
  availableSize?: number | null;
}

export interface CatalogView {
  source: "bundled" | "cache" | "remote";
  fetchedAt?: number | null;
  schemaVersion: number;
  games: GameView[];
}

export interface AppConfig {
  installRoot?: string | null;
  catalogUrl?: string | null;
  downloadMirror?: string | null;
  proxy?: string | null;
  concurrency: number;
  autoCheckUpdate: boolean;
  launcherAutoUpdate: boolean;
}

export interface InstalledVersion {
  version: string;
  path: string;
  installedAt: number;
  sha256?: string | null;
}

export interface GameInstall {
  currentVersion?: string | null;
  installed: InstalledVersion[];
}

export interface InstallState {
  games: Record<string, GameInstall>;
}

export interface LaunchOutcome {
  pid: number;
  version: string;
}

export type TaskPhase = "queued" | "download" | "verify" | "extract";

export interface TaskProgress {
  taskId: string;
  gameId: string;
  phase: TaskPhase;
  received: number;
  total: number | null;
  speedBps: number;
}

export interface TaskDone {
  taskId: string;
  gameId: string;
  ok: boolean;
  error?: string | null;
  path?: string | null;
}
