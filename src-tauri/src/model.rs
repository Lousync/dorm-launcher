use serde::{Deserialize, Serialize};

/// 当前支持的清单协议版本。
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

// ───────────────────────────── 清单（远程 / 内置） ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub schema_version: u32,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub games: Vec<Game>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub cover: Option<String>,
    #[serde(default)]
    pub screenshots: Vec<String>,
    pub source: VersionSource,
    pub executable: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub user_data_note: Option<String>,
    #[serde(default)]
    pub latest: Option<GameVersion>,
    #[serde(default)]
    pub versions: Vec<GameVersion>,
}

/// 版本从哪来：GitHub Releases 动态解析，或写死直链。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum VersionSource {
    GithubReleases {
        repo: String,
        #[serde(default)]
        tag_prefix: Option<String>,
        asset_pattern: String,
    },
    Direct {
        url: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameVersion {
    pub version: String,
    pub url: String,
    pub sha256: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

// ───────────────────────────── 面向界面的视图 ─────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GameStatus {
    /// 从未安装。
    NotInstalled,
    /// 已安装且为最新（或无法判定最新版）。
    UpToDate,
    /// 已安装但有新版本可用。
    UpdateAvailable,
}

/// 清单里的游戏 + 本地安装状态，合并后交给前端渲染。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameView {
    pub id: String,
    pub name: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub cover: Option<String>,
    pub screenshots: Vec<String>,
    pub executable: String,
    pub args: Vec<String>,
    pub user_data_note: Option<String>,
    pub latest_version: Option<String>,
    pub installed_version: Option<String>,
    pub installed_versions: Vec<String>,
    /// 可供安装 / 回退的历史版本（详情页版本历史）。
    pub versions: Vec<GameVersion>,
    pub status: GameStatus,
    pub install_path: Option<String>,
    pub available_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CatalogView {
    /// 清单来源：`bundled` | `cache` | `remote`。
    pub source: String,
    pub fetched_at: Option<u64>,
    pub schema_version: u32,
    pub games: Vec<GameView>,
}

/// 安装 / 下载任务的进度事件（`task:progress`）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskProgress {
    pub task_id: String,
    pub game_id: String,
    /// `download` | `verify` | `extract`。
    pub phase: String,
    pub received: u64,
    pub total: Option<u64>,
    pub speed_bps: f64,
}

/// 任务结束事件（`task:done`）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDone {
    pub task_id: String,
    pub game_id: String,
    pub ok: bool,
    pub error: Option<String>,
    pub path: Option<String>,
}

/// 启动游戏的返回值。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchOutcome {
    pub pid: u32,
    pub version: String,
}

/// 比较「已装版本」与「最新版本」，得出展示状态。
/// 版本号按 semver 比较；解析失败时退回字符串相等判断。
pub fn compute_status(installed: Option<&str>, latest: Option<&str>) -> GameStatus {
    let Some(installed) = installed else {
        return GameStatus::NotInstalled;
    };
    let Some(latest) = latest else {
        return GameStatus::UpToDate;
    };
    if installed == latest {
        return GameStatus::UpToDate;
    }
    match (
        semver::Version::parse(installed),
        semver::Version::parse(latest),
    ) {
        (Ok(cur), Ok(new)) if cur < new => GameStatus::UpdateAvailable,
        (Ok(cur), Ok(new)) if cur >= new => GameStatus::UpToDate,
        _ => GameStatus::UpToDate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_not_installed() {
        assert_eq!(
            compute_status(None, Some("1.0.0")),
            GameStatus::NotInstalled
        );
    }

    #[test]
    fn status_up_to_date() {
        assert_eq!(
            compute_status(Some("1.0.0"), Some("1.0.0")),
            GameStatus::UpToDate
        );
        assert_eq!(
            compute_status(Some("1.2.0"), Some("1.1.0")),
            GameStatus::UpToDate
        );
    }

    #[test]
    fn status_update_available() {
        assert_eq!(
            compute_status(Some("0.12.0"), Some("0.13.1")),
            GameStatus::UpdateAvailable
        );
        // 未知最新版时视为最新，不误报更新。
        assert_eq!(compute_status(Some("0.12.0"), None), GameStatus::UpToDate);
    }
}
