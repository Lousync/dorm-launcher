use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppResult;

/// 本地安装状态，写到 `state.json`。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct InstallState {
    pub games: HashMap<String, GameInstall>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GameInstall {
    pub current_version: Option<String>,
    pub installed: Vec<InstalledVersion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledVersion {
    pub version: String,
    pub path: String,
    pub installed_at: u64,
    #[serde(default)]
    pub sha256: Option<String>,
}

pub fn load(path: &Path) -> AppResult<InstallState> {
    if !path.exists() {
        return Ok(InstallState::default());
    }
    let text = std::fs::read_to_string(path)?;
    if text.trim().is_empty() {
        return Ok(InstallState::default());
    }
    Ok(serde_json::from_str(&text)?)
}

pub fn save(path: &Path, st: &InstallState) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(st)?)?;
    Ok(())
}

impl InstallState {
    /// 切换到某个已安装版本；成功返回 true。
    pub fn set_current(&mut self, id: &str, version: &str) -> bool {
        match self.games.get_mut(id) {
            Some(entry) if entry.installed.iter().any(|v| v.version == version) => {
                entry.current_version = Some(version.to_string());
                true
            }
            _ => false,
        }
    }

    /// 移除某版本；若删的是当前版本则回退到其余已装版本；没有剩余则整条删除。
    pub fn remove_version(&mut self, id: &str, version: &str) {
        let mut drop_game = false;
        if let Some(entry) = self.games.get_mut(id) {
            entry.installed.retain(|v| v.version != version);
            if entry.current_version.as_deref() == Some(version) {
                entry.current_version = entry.installed.first().map(|v| v.version.clone());
            }
            drop_game = entry.installed.is_empty() && entry.current_version.is_none();
        }
        if drop_game {
            self.games.remove(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with(id: &str, versions: &[&str], current: &str) -> InstallState {
        let mut st = InstallState::default();
        let entry = st.games.entry(id.to_string()).or_default();
        for v in versions {
            entry.installed.push(InstalledVersion {
                version: v.to_string(),
                path: format!("C:/games/{id}/{v}"),
                installed_at: 0,
                sha256: None,
            });
        }
        entry.current_version = Some(current.to_string());
        st
    }

    #[test]
    fn set_current_only_for_installed() {
        let mut st = state_with("g", &["1.0.0", "1.1.0"], "1.0.0");
        assert!(st.set_current("g", "1.1.0"));
        assert_eq!(st.games["g"].current_version.as_deref(), Some("1.1.0"));
        // 未安装的版本不能切
        assert!(!st.set_current("g", "9.9.9"));
        assert!(!st.set_current("nope", "1.0.0"));
    }

    #[test]
    fn remove_current_rolls_back() {
        let mut st = state_with("g", &["1.0.0", "1.1.0"], "1.1.0");
        st.remove_version("g", "1.1.0");
        assert_eq!(st.games["g"].current_version.as_deref(), Some("1.0.0"));
        assert_eq!(st.games["g"].installed.len(), 1);
    }

    #[test]
    fn removing_last_version_drops_game() {
        let mut st = state_with("g", &["1.0.0"], "1.0.0");
        st.remove_version("g", "1.0.0");
        assert!(!st.games.contains_key("g"));
    }
}
