use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppResult;

/// 用户设置。字段全部可选 / 有默认值，写到 `config.json`。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    /// 游戏安装根目录；`None` = 用默认（`%LOCALAPPDATA%\DormGameHub`）。
    pub install_root: Option<String>,
    /// 远程清单 URL；`None` = 只用内置 / 缓存清单。
    pub catalog_url: Option<String>,
    /// 下载 / 清单的 GitHub 加速前缀，形如 `https://gh-proxy.com/`；
    /// 只对 GitHub 域名生效，也可用 `{url}` 占位符自定义拼接。
    /// 默认给一个可用镜像，国内网络无需额外配置；可留空直连。
    pub download_mirror: Option<String>,
    /// 代理（形如 `http://127.0.0.1:7890`）。
    pub proxy: Option<String>,
    /// 并发下载数。
    pub concurrency: u32,
    /// 启动时自动检查更新。
    pub auto_check_update: bool,
    /// 启动器自身自动更新。
    pub launcher_auto_update: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            install_root: None,
            catalog_url: None,
            download_mirror: Some("https://gh-proxy.com/".to_string()),
            proxy: None,
            concurrency: 2,
            auto_check_update: true,
            launcher_auto_update: true,
        }
    }
}

pub fn load(path: &Path) -> AppResult<AppConfig> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let text = std::fs::read_to_string(path)?;
    if text.trim().is_empty() {
        return Ok(AppConfig::default());
    }
    Ok(serde_json::from_str(&text)?)
}

pub fn save(path: &Path, cfg: &AppConfig) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(cfg)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_sane() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.concurrency, 2);
        assert!(cfg.auto_check_update);
        assert!(cfg.install_root.is_none());
    }

    #[test]
    fn load_missing_returns_default() {
        let path = std::env::temp_dir().join("dorm-launcher-missing-config.json");
        let _ = std::fs::remove_file(&path);
        let cfg = load(&path).unwrap();
        assert_eq!(cfg.concurrency, 2);
    }

    #[test]
    fn save_then_load_roundtrip() {
        let path = std::env::temp_dir().join("dorm-launcher-config-roundtrip.json");
        let cfg = AppConfig {
            install_root: Some("D:\\Games".to_string()),
            concurrency: 4,
            ..AppConfig::default()
        };
        save(&path, &cfg).unwrap();
        let back = load(&path).unwrap();
        assert_eq!(back.install_root.as_deref(), Some("D:\\Games"));
        assert_eq!(back.concurrency, 4);
        let _ = std::fs::remove_file(&path);
    }
}
