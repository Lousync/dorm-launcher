import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, AppInfo, CatalogView, InstallState, LaunchOutcome } from "./types";

/** 后端 command 的类型化封装。前端不直接碰系统能力，一律走这里。 */

export const getAppInfo = () => invoke<AppInfo>("app_info");

export const getConfig = () => invoke<AppConfig>("get_config");

export const setConfig = (config: AppConfig) =>
  invoke<AppConfig>("set_config", { config });

export const getDefaultInstallRoot = () =>
  invoke<string>("default_install_root");

export const getAppDataDir = () => invoke<string>("app_data_dir");

export const getState = () => invoke<InstallState>("get_state");

export const getCatalog = (refresh = false) =>
  invoke<CatalogView>("get_catalog", { refresh });

export const installGame = (id: string, version: string) =>
  invoke<string>("install_game", { id, version });

export const cancelTask = (taskId: string) =>
  invoke<string>("cancel_task", { taskId });

export const launchGame = (id: string) =>
  invoke<LaunchOutcome>("launch_game", { id });

export const setCurrentVersion = (id: string, version: string) =>
  invoke<void>("set_current_version", { id, version });

export const uninstallGame = (id: string, version: string) =>
  invoke<void>("uninstall_game", { id, version });
