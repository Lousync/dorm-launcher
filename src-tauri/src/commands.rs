use std::path::Path;

use tauri::AppHandle;

use crate::catalog;
use crate::config::{self, AppConfig};
use crate::error::{AppError, AppResult};
use crate::model::{AppInfo, CatalogView, Game, GameView, LaunchOutcome};
use crate::paths;
use crate::state::{self, InstallState};

#[tauri::command]
pub fn app_info() -> AppInfo {
    AppInfo {
        name: "宿舍游戏库".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[tauri::command]
pub fn get_config() -> AppResult<AppConfig> {
    config::load(&paths::config_path()?)
}

#[tauri::command]
pub fn set_config(config: AppConfig) -> AppResult<AppConfig> {
    config::save(&paths::config_path()?, &config)?;
    Ok(config)
}

/// 默认安装根目录（供前端在设置为空时展示）。
#[tauri::command]
pub fn default_install_root() -> AppResult<String> {
    Ok(paths::default_install_root()?.to_string_lossy().to_string())
}

/// 应用数据目录（配置 / 状态 / 日志所在）。
#[tauri::command]
pub fn app_data_dir() -> AppResult<String> {
    Ok(paths::app_data_dir()?.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_state() -> AppResult<InstallState> {
    state::load(&paths::state_path()?)
}

/// 返回清单 + 本地状态合并后的游戏列表。`refresh = true` 时强制拉远程。
#[tauri::command]
pub async fn get_catalog(refresh: bool) -> AppResult<CatalogView> {
    let cfg = config::load(&paths::config_path()?)?;
    let st = state::load(&paths::state_path()?)?;
    let root = paths::install_root(&cfg)?;

    let (manifest, source, fetched_at) = catalog::resolve(&cfg, refresh).await?;
    let games = manifest
        .games
        .iter()
        .map(|game| build_view(game, &st, &root))
        .collect();

    Ok(CatalogView {
        source,
        fetched_at,
        schema_version: manifest.schema_version,
        games,
    })
}

/// 把指定版本加入下载队列（立即返回 `taskId`，进度走 `task:progress` 事件）。
#[tauri::command]
pub fn install_game(app: AppHandle, id: String, version: String) -> String {
    crate::downloads::enqueue(&app, id, version)
}

/// 取消排队的 / 进行中的下载任务（按 `taskId`）；返回 `cancelling` / `dequeued` / `notFound`。
#[tauri::command]
pub fn cancel_task(app: AppHandle, task_id: String) -> String {
    crate::downloads::cancel(&app, &task_id)
}

fn build_view(game: &Game, st: &InstallState, root: &Path) -> GameView {
    let install = st.games.get(&game.id);

    let latest_version = game.latest.as_ref().map(|v| v.version.clone());
    let installed_version = install.and_then(|i| i.current_version.clone());
    let installed_versions = install
        .map(|i| i.installed.iter().map(|v| v.version.clone()).collect())
        .unwrap_or_default();

    let status =
        crate::model::compute_status(installed_version.as_deref(), latest_version.as_deref());

    let install_path = installed_version.as_ref().map(|v| {
        paths::version_dir(root, &game.id, v)
            .to_string_lossy()
            .to_string()
    });

    let versions = if game.versions.is_empty() {
        game.latest.clone().into_iter().collect()
    } else {
        game.versions.clone()
    };

    GameView {
        id: game.id.clone(),
        name: game.name.clone(),
        summary: game.summary.clone(),
        description: game.description.clone(),
        cover: game.cover.clone(),
        screenshots: game.screenshots.clone(),
        executable: game.executable.clone(),
        args: game.args.clone(),
        user_data_note: game.user_data_note.clone(),
        latest_version,
        installed_version,
        installed_versions,
        versions,
        status,
        install_path,
        available_size: game.latest.as_ref().and_then(|v| v.size),
    }
}

/// 启动「当前版本」的游戏可执行文件（工作目录 = 版本目录）。
#[tauri::command]
pub async fn launch_game(id: String) -> AppResult<LaunchOutcome> {
    let cfg = config::load(&paths::config_path()?)?;
    let root = paths::install_root(&cfg)?;

    let st = state::load(&paths::state_path()?)?;
    let version = st
        .games
        .get(&id)
        .and_then(|g| g.current_version.clone())
        .ok_or_else(|| AppError::Other(format!("{id} 还没有可启动的版本")))?;

    let (manifest, _source, _) = catalog::resolve(&cfg, false).await?;
    let game = manifest
        .games
        .iter()
        .find(|g| g.id == id)
        .ok_or_else(|| AppError::Manifest(format!("清单里没有游戏 {id}")))?;

    let dir = paths::version_dir(&root, &id, &version);
    let exe = dir.join(&game.executable);
    if !exe.exists() {
        return Err(AppError::Other(format!(
            "找不到可执行文件：{}",
            exe.display()
        )));
    }

    let mut cmd = std::process::Command::new(&exe);
    cmd.current_dir(&dir);
    cmd.args(&game.args);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let child = cmd
        .spawn()
        .map_err(|e| AppError::Other(format!("启动失败：{e}")))?;

    Ok(LaunchOutcome {
        pid: child.id(),
        version,
    })
}

/// 切换 / 回退到某个已安装的版本。
#[tauri::command]
pub fn set_current_version(id: String, version: String) -> AppResult<()> {
    let path = paths::state_path()?;
    let mut st = state::load(&path)?;
    if !st.set_current(&id, &version) {
        return Err(AppError::Other(format!("{id} 没有已安装的版本 {version}")));
    }
    state::save(&path, &st)?;
    Ok(())
}

/// 卸载某个已安装版本（删目录 + 更新状态；若删的是当前版本则回退到其余版本）。
#[tauri::command]
pub fn uninstall_game(id: String, version: String) -> AppResult<()> {
    let cfg = config::load(&paths::config_path()?)?;
    let root = paths::install_root(&cfg)?;
    let dir = paths::version_dir(&root, &id, &version);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }

    let path = paths::state_path()?;
    let mut st = state::load(&path)?;
    st.remove_version(&id, &version);
    state::save(&path, &st)?;
    Ok(())
}
