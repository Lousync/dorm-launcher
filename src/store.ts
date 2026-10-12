import { create } from "zustand";
import { listen } from "@tauri-apps/api/event";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import * as api from "./api";
import type {
  AppConfig,
  AppInfo,
  CatalogView,
  TaskDone,
  TaskPhase,
  TaskProgress,
} from "./types";

export type View = "library" | "downloads" | "detail" | "settings";

export interface TaskState {
  taskId: string;
  gameId: string;
  version: string;
  phase: TaskPhase | "done";
  received: number;
  total: number | null;
  speed: number;
  finished: boolean;
  ok: boolean | null;
  error: string | null;
  path: string | null;
}

function splitTaskId(taskId: string): { gameId: string; version: string } {
  const at = taskId.indexOf("@");
  if (at < 0) return { gameId: taskId, version: "" };
  return { gameId: taskId.slice(0, at), version: taskId.slice(at + 1) };
}

function initTask(taskId: string): TaskState {
  const { gameId, version } = splitTaskId(taskId);
  return {
    taskId,
    gameId,
    version,
    phase: "queued",
    received: 0,
    total: null,
    speed: 0,
    finished: false,
    ok: null,
    error: null,
    path: null,
  };
}

interface LauncherState {
  appInfo: AppInfo | null;
  config: AppConfig | null;
  defaultInstallRoot: string | null;
  appDataDir: string | null;
  catalog: CatalogView | null;
  loading: boolean;
  refreshing: boolean;
  error: string | null;

  view: View;
  selectedId: string | null;
  toast: string | null;

  tasks: Record<string, TaskState>;

  appUpdate: AppUpdateInfo | null;
  updateStatus: "idle" | "downloading" | "ready" | "error";
  updateProgress: number;

  bootstrap: () => Promise<void>;
  refreshCatalog: () => Promise<void>;
  reloadCatalog: () => Promise<void>;
  saveConfig: (patch: Partial<AppConfig>) => Promise<boolean>;
  setView: (view: View) => void;
  openDetail: (id: string) => void;
  showToast: (message: string) => void;
  closeToast: () => void;
  clearError: () => void;

  startInstall: (
    gameId: string,
    version: string,
    opts?: { latest?: boolean },
  ) => Promise<void>;
  cancelInstall: (taskId: string) => Promise<void>;
  clearFinished: () => void;

  launchGame: (gameId: string) => Promise<void>;
  setCurrentVersion: (gameId: string, version: string) => Promise<void>;
  uninstallGame: (gameId: string, version: string) => Promise<void>;

  checkAppUpdate: (silent?: boolean) => Promise<void>;
  installAppUpdate: () => Promise<void>;
}

let listenersReady = false;
let pendingUpdate: Update | null = null;

export interface AppUpdateInfo {
  version: string;
  notes: string | null;
  date: string | null;
}

export const useLauncher = create<LauncherState>((set, get) => ({
  appInfo: null,
  config: null,
  defaultInstallRoot: null,
  appDataDir: null,
  catalog: null,
  loading: false,
  refreshing: false,
  error: null,

  view: "library",
  selectedId: null,
  toast: null,

  tasks: {},

  appUpdate: null,
  updateStatus: "idle",
  updateProgress: 0,

  async bootstrap() {
    // 支持 #downloads / #settings 直达某一屏（也便于测试/截图）。
    const hash = window.location.hash.replace(/^#\/?/, "");
    if (hash === "downloads" || hash === "settings") {
      set({ view: hash as View });
    }

    set({ loading: true, error: null });
    try {
      const [appInfo, config, defaultInstallRoot, appDataDir, catalog] =
        await Promise.all([
          api.getAppInfo(),
          api.getConfig(),
          api.getDefaultInstallRoot(),
          api.getAppDataDir(),
          api.getCatalog(false),
        ]);
      set({ appInfo, config, defaultInstallRoot, appDataDir, catalog });

      // 后台自动刷新远程清单（不阻塞首屏）。
      if (config.autoCheckUpdate && config.catalogUrl?.trim()) {
        api
          .getCatalog(true)
          .then((fresh) => set({ catalog: fresh }))
          .catch(() => {});
      }

      // 后台检查启动器自更新。
      if (config.launcherAutoUpdate) {
        get().checkAppUpdate(true);
      }
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ loading: false });
    }

    if (!listenersReady) {
      listenersReady = true;
      await listen<TaskProgress>("task:progress", (e) => {
        const p = e.payload;
        set((s) => {
          const prev = s.tasks[p.taskId] ?? initTask(p.taskId);
          return {
            tasks: {
              ...s.tasks,
              [p.taskId]: {
                ...prev,
                phase: p.phase,
                received: p.received,
                total: p.total,
                speed: p.speedBps,
              },
            },
          };
        });
      });
      await listen<TaskDone>("task:done", (e) => {
        const d = e.payload;
        set((s) => {
          const prev = s.tasks[d.taskId] ?? initTask(d.taskId);
          return {
            tasks: {
              ...s.tasks,
              [d.taskId]: {
                ...prev,
                phase: "done",
                finished: true,
                ok: d.ok,
                error: d.error ?? null,
                path: d.path ?? prev.path,
              },
            },
          };
        });
        if (d.ok) {
          get().showToast("安装完成");
          get().reloadCatalog();
        } else if (d.error && d.error !== "已取消") {
          get().showToast(`安装失败：${d.error}`);
        }
      });
    }
  },

  async refreshCatalog() {
    set({ refreshing: true, error: null });
    try {
      const catalog = await api.getCatalog(true);
      set({ catalog });
    } catch (e) {
      set({ error: String(e) });
    } finally {
      set({ refreshing: false });
    }
  },

  async reloadCatalog() {
    try {
      const catalog = await api.getCatalog(false);
      set({ catalog });
    } catch (e) {
      set({ error: String(e) });
    }
  },

  async saveConfig(patch) {
    const current = get().config;
    if (!current) return false;
    const next: AppConfig = { ...current, ...patch };
    try {
      const saved = await api.setConfig(next);
      set({ config: saved });
      return true;
    } catch (e) {
      set({ error: String(e) });
      return false;
    }
  },

  setView(view) {
    set({ view });
  },

  openDetail(id) {
    set({ view: "detail", selectedId: id });
  },

  showToast(message) {
    set({ toast: message });
  },

  closeToast() {
    set({ toast: null });
  },

  clearError() {
    set({ error: null });
  },

  async startInstall(gameId, version, opts) {
    // 安装前先强制刷新源头：拉远程清单 + 动态解析最新 release，
    // 保证拿到真正的「最新版」以及最新的下载链接 / 校验值。
    set({ refreshing: true, error: null });
    try {
      const catalog = await api.getCatalog(true);
      set({ catalog });
    } catch {
      // 刷新失败（离线等）不阻断安装，退回现有 / 缓存清单继续。
    } finally {
      set({ refreshing: false });
    }

    // 决定真正要装的版本：`latest` 意图 = 装刷新后的最新版；
    // 指定历史版本仍存在则尊重用户选择，已不存在时退回最新版。
    const game = get().catalog?.games.find((g) => g.id === gameId);
    const freshLatest = game?.latestVersion ?? null;
    const requestedExists =
      !!game &&
      (game.latestVersion === version ||
        game.versions.some((v) => v.version === version));
    const target =
      (opts?.latest || !requestedExists) && freshLatest ? freshLatest : version;

    const taskId = `${gameId}@${target}`;
    set((s) => ({
      tasks: { ...s.tasks, [taskId]: initTask(taskId) },
    }));
    try {
      await api.installGame(gameId, target);
      get().showToast("已加入下载队列");
    } catch (e) {
      set((s) => ({
        tasks: {
          ...s.tasks,
          [taskId]: { ...initTask(taskId), finished: true, ok: false, error: String(e) },
        },
      }));
    }
  },

  async cancelInstall(taskId) {
    try {
      await api.cancelTask(taskId);
    } catch {
      /* 忽略 */
    }
  },

  clearFinished() {
    set((s) => {
      const tasks: Record<string, TaskState> = {};
      for (const [k, v] of Object.entries(s.tasks)) {
        if (!v.finished) tasks[k] = v;
      }
      return { tasks };
    });
  },

  async launchGame(gameId) {
    try {
      const r = await api.launchGame(gameId);
      get().showToast(`已启动（PID ${r.pid}）`);
    } catch (e) {
      get().showToast(`启动失败：${e}`);
    }
  },

  async setCurrentVersion(gameId, version) {
    try {
      await api.setCurrentVersion(gameId, version);
      get().showToast(`已切换到 ${version}`);
      await get().reloadCatalog();
    } catch (e) {
      get().showToast(String(e));
    }
  },

  async uninstallGame(gameId, version) {
    try {
      await api.uninstallGame(gameId, version);
      get().showToast(`已卸载 ${version}`);
      await get().reloadCatalog();
    } catch (e) {
      get().showToast(String(e));
    }
  },

  async checkAppUpdate(silent = false) {
    const proxy = get().config?.proxy?.trim() || undefined;
    try {
      const update = await check({ proxy });
      if (update) {
        pendingUpdate = update;
        set({
          appUpdate: {
            version: update.version,
            notes: update.body ?? null,
            date: update.date ?? null,
          },
        });
        get().showToast(`发现启动器新版本 v${update.version}`);
      } else {
        pendingUpdate = null;
        set({ appUpdate: null });
        if (!silent) get().showToast("启动器已是最新版本");
      }
    } catch (e) {
      pendingUpdate = null;
      set({ appUpdate: null });
      if (!silent) get().showToast(`检查更新失败：${e}`);
    }
  },

  async installAppUpdate() {
    if (!pendingUpdate) {
      await get().checkAppUpdate(false);
    }
    if (!pendingUpdate) return;

    set({ updateStatus: "downloading", updateProgress: 0 });
    try {
      let total = 0;
      let downloaded = 0;
      await pendingUpdate.downloadAndInstall((event) => {
        if (event.event === "Started") {
          total = event.data.contentLength ?? 0;
        } else if (event.event === "Progress") {
          downloaded += event.data.chunkLength;
          if (total > 0) {
            set({ updateProgress: Math.min(100, Math.round((downloaded / total) * 100)) });
          }
        } else if (event.event === "Finished") {
          set({ updateProgress: 100 });
        }
      });
      set({ updateStatus: "ready" });
      get().showToast("更新已安装，正在重启…");
      await relaunch();
    } catch (e) {
      set({ updateStatus: "error" });
      get().showToast(`更新失败：${e}`);
    }
  },
}));
