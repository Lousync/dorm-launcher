use serde::Serialize;

/// 统一错误类型。实现 `Serialize`，让 `#[tauri::command]` 能把它回传给前端。
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("文件读写失败：{0}")]
    Io(#[from] std::io::Error),
    #[error("数据格式错误：{0}")]
    Json(#[from] serde_json::Error),
    #[error("网络请求失败：{0}")]
    Http(#[from] reqwest::Error),
    #[error("清单协议错误：{0}")]
    Manifest(String),
    #[error("路径错误：{0}")]
    Path(String),
    // 供尚未实现的功能（下载 / 安装 / 启动，M2/M3）使用。
    #[allow(dead_code)]
    #[error("未实现：{0}")]
    NotImplemented(String),
    #[allow(dead_code)]
    #[error("{0}")]
    Other(String),
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
