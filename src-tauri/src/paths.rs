use std::path::{Path, PathBuf};

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};

/// 应用数据目录（小文件：配置 / 状态 / 清单缓存），位于 `%APPDATA%\DormGameHub`。
pub fn app_data_dir() -> AppResult<PathBuf> {
    let base = std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| AppError::Path("找不到 %APPDATA% 环境变量".into()))?;
    let dir = base.join("DormGameHub");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

pub fn config_path() -> AppResult<PathBuf> {
    Ok(app_data_dir()?.join("config.json"))
}

pub fn state_path() -> AppResult<PathBuf> {
    Ok(app_data_dir()?.join("state.json"))
}

pub fn catalog_cache_path() -> AppResult<PathBuf> {
    Ok(app_data_dir()?.join("catalog-cache.json"))
}

/// 默认游戏安装根目录，位于 `%LOCALAPPDATA%\DormGameHub`（可被配置覆盖）。
pub fn default_install_root() -> AppResult<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .ok_or_else(|| AppError::Path("找不到 %LOCALAPPDATA% 环境变量".into()))?;
    let dir = base.join("DormGameHub");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// 生效的安装根目录：配置里指定则用配置，否则用默认。
pub fn install_root(cfg: &AppConfig) -> AppResult<PathBuf> {
    match cfg.install_root.as_deref() {
        Some(s) if !s.trim().is_empty() => Ok(PathBuf::from(s)),
        _ => default_install_root(),
    }
}

pub fn games_dir(root: &Path) -> PathBuf {
    root.join("games")
}

pub fn game_dir(root: &Path, id: &str) -> PathBuf {
    games_dir(root).join(id)
}

pub fn versions_dir(root: &Path, id: &str) -> PathBuf {
    game_dir(root, id).join("versions")
}

pub fn version_dir(root: &Path, id: &str, version: &str) -> PathBuf {
    versions_dir(root, id).join(version)
}
