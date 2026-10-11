use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::error::{AppError, AppResult};
use crate::model::{GameVersion, MANIFEST_SCHEMA_VERSION, Manifest, VersionSource};
use crate::paths;

/// 内置兜底清单：直接引用仓库根的 `catalog/manifest.json`，
/// 与服务端托管的**是同一份文件**（单一来源，避免本地/远程分叉）。
/// 远程拉取失败时用它，保证首次启动不空白。
const BUNDLED_MANIFEST: &str = include_str!("../../catalog/manifest.json");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogCache {
    fetched_at: u64,
    manifest: Manifest,
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn bundled_manifest() -> AppResult<Manifest> {
    let manifest: Manifest = serde_json::from_str(BUNDLED_MANIFEST)?;
    validate(&manifest)?;
    Ok(manifest)
}

fn validate(manifest: &Manifest) -> AppResult<()> {
    if manifest.schema_version != MANIFEST_SCHEMA_VERSION {
        return Err(AppError::Manifest(format!(
            "不支持的清单版本 {}（本启动器期望 {}）",
            manifest.schema_version, MANIFEST_SCHEMA_VERSION
        )));
    }
    Ok(())
}

fn load_cache(path: &Path) -> Option<CatalogCache> {
    if !path.exists() {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<CatalogCache>(&text).ok()
}

fn save_cache(path: &Path, manifest: &Manifest, fetched_at: u64) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let cache = CatalogCache {
        fetched_at,
        manifest: manifest.clone(),
    };
    std::fs::write(path, serde_json::to_string_pretty(&cache)?)?;
    Ok(())
}

async fn fetch_remote(url: &str, proxy: Option<&str>, mirror: Option<&str>) -> AppResult<Manifest> {
    let mut builder = reqwest::Client::builder()
        .user_agent(concat!("dorm-launcher/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(20));
    if let Some(p) = proxy.filter(|p| !p.trim().is_empty()) {
        builder = builder.proxy(reqwest::Proxy::all(p)?);
    }
    let client = builder.build()?;
    let target = apply_mirror(url, mirror);
    let text = client
        .get(&target)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let manifest: Manifest = serde_json::from_str(&text)?;
    validate(&manifest)?;
    Ok(manifest)
}

/// 给 GitHub 链接套加速前缀；非 GitHub 链接原样返回。
/// 前缀可用 `{url}` 占位符自定义拼接，否则直接前缀拼接。
pub fn apply_mirror(url: &str, mirror: Option<&str>) -> String {
    match mirror.map(str::trim).filter(|m| !m.is_empty()) {
        Some(m) if url.contains("github.com") || url.contains("githubusercontent.com") => {
            if m.contains("{url}") {
                m.replace("{url}", url)
            } else {
                format!("{}/{}", m.trim_end_matches('/'), url)
            }
        }
        _ => url.to_string(),
    }
}

/// 解析出要用的清单。优先级：远程（刷新 / 无缓存）→ 缓存 → 内置；
/// 刷新时还会对 `github-releases` 类游戏做一次动态解析（见 `hydrate_manifest`）。
/// 返回值：`(清单, 来源, 拉取时间)`。
pub async fn resolve(cfg: &AppConfig, refresh: bool) -> AppResult<(Manifest, String, Option<u64>)> {
    let cache_path = paths::catalog_cache_path()?;
    let cached = load_cache(&cache_path);
    let url = cfg.catalog_url.as_deref().filter(|u| !u.trim().is_empty());

    let (mut manifest, source, mut fetched_at) = match url {
        // 首次（无缓存）或显式刷新时才走网络。
        Some(url) if refresh || cached.is_none() => {
            match fetch_remote(url, cfg.proxy.as_deref(), cfg.download_mirror.as_deref()).await {
                Ok(manifest) => (manifest, "remote".to_string(), Some(now_secs())),
                Err(err) => {
                    eprintln!("[catalog] 远程清单拉取失败，改用本地：{err}");
                    fallback_manifest(cached)?
                }
            }
        }
        _ => fallback_manifest(cached)?,
    };

    if refresh {
        hydrate_manifest(&mut manifest, cfg.proxy.as_deref()).await;
        fetched_at = Some(now_secs());
        // 缓存带动态解析结果的清单，供后续离线 / 快速读取。
        let _ = save_cache(&cache_path, &manifest, fetched_at.unwrap_or_else(now_secs));
    }

    Ok((manifest, source, fetched_at))
}

fn fallback_manifest(cached: Option<CatalogCache>) -> AppResult<(Manifest, String, Option<u64>)> {
    match cached {
        Some(cache) => Ok((cache.manifest, "cache".to_string(), Some(cache.fetched_at))),
        None => Ok((bundled_manifest()?, "bundled".to_string(), None)),
    }
}

/// 对 `github-releases` 类游戏查 GitHub API，把带资产的 release 解析成版本列表；
/// 解析不到（网络失败 / 无匹配资产 / 无摘要可校验）时保留清单里写死的版本作兜底。
async fn hydrate_manifest(manifest: &mut Manifest, proxy: Option<&str>) {
    let targets: Vec<(usize, String, Option<String>, String)> = manifest
        .games
        .iter()
        .enumerate()
        .filter_map(|(i, g)| match &g.source {
            VersionSource::GithubReleases {
                repo,
                tag_prefix,
                asset_pattern,
            } => Some((i, repo.clone(), tag_prefix.clone(), asset_pattern.clone())),
            VersionSource::Direct { .. } => None,
        })
        .collect();

    for (i, repo, tag_prefix, pattern) in targets {
        match crate::github::resolve_releases(&repo, tag_prefix.as_deref(), &pattern, proxy).await {
            Ok(list) if !list.is_empty() => {
                let pinned = manifest.games[i].versions.clone();
                // 解析结果若没有摘要，则回填清单里写死的 sha256；仍无摘要的版本丢弃（无法校验）。
                let merged: Vec<GameVersion> = list
                    .into_iter()
                    .map(|mut v| {
                        if v.sha256.is_empty()
                            && let Some(p) = pinned.iter().find(|p| p.version == v.version)
                        {
                            v.sha256 = p.sha256.clone();
                        }
                        v
                    })
                    .filter(|v| !v.sha256.is_empty())
                    .collect();

                if !merged.is_empty() {
                    manifest.games[i].latest = merged.first().cloned();
                    manifest.games[i].versions = merged;
                }
            }
            Ok(_) => {}
            Err(err) => eprintln!("[catalog] {repo} 的 release 动态解析失败：{err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_manifest_parses_and_has_game() {
        let manifest = bundled_manifest().expect("内置清单应能解析");
        assert_eq!(manifest.schema_version, MANIFEST_SCHEMA_VERSION);
        assert!(manifest.games.iter().any(|g| g.id == "dorm-monopoly"));
    }

    #[test]
    fn bundled_latest_has_valid_sha256_and_url() {
        let manifest = bundled_manifest().unwrap();
        let game = manifest
            .games
            .iter()
            .find(|g| g.id == "dorm-monopoly")
            .unwrap();
        let latest = game.latest.as_ref().expect("应配置 latest");
        assert_eq!(latest.sha256.len(), 64, "sha256 应为 64 位十六进制");
        assert!(latest.url.starts_with("https://github.com/"));
        assert!(!game.executable.is_empty());
    }

    #[test]
    fn rejects_unknown_schema_version() {
        let mut manifest = bundled_manifest().unwrap();
        manifest.schema_version = 999;
        assert!(validate(&manifest).is_err());
    }

    #[test]
    fn mirror_prefixes_github_only() {
        let gh = "https://github.com/u/r/releases/download/v1/a.zip";
        assert_eq!(
            apply_mirror(gh, Some("https://gh-proxy.com/")),
            "https://gh-proxy.com/https://github.com/u/r/releases/download/v1/a.zip"
        );
        // 非 GitHub 链接不受影响
        let other = "https://example.com/a.zip";
        assert_eq!(apply_mirror(other, Some("https://gh-proxy.com/")), other);
        // 占位符形式
        assert_eq!(
            apply_mirror(gh, Some("https://m/?u={url}")),
            "https://m/?u=https://github.com/u/r/releases/download/v1/a.zip"
        );
        // 未配置镜像则原样
        assert_eq!(apply_mirror(gh, None), gh);
    }

    #[test]
    fn cache_roundtrip() {
        let dir = std::env::temp_dir().join("dorm-launcher-cache-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("catalog-cache.json");

        let manifest = bundled_manifest().unwrap();
        save_cache(&path, &manifest, 12345).unwrap();
        let cache = load_cache(&path).expect("应能读回缓存");
        assert_eq!(cache.fetched_at, 12345);
        assert_eq!(cache.manifest.games.len(), manifest.games.len());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parses_multiple_games() {
        let json = r#"{
          "schemaVersion": 1,
          "games": [
            { "id":"a","name":"A","source":{"type":"direct","url":"https://e/a.zip"},"executable":"a.exe" },
            { "id":"b","name":"B","source":{"type":"github-releases","repo":"u/r","assetPattern":"^b-(.*)$"},"executable":"b.exe" }
          ]
        }"#;
        let manifest: Manifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.games.len(), 2);
        assert_eq!(manifest.games[1].executable, "b.exe");
        assert!(matches!(
            manifest.games[0].source,
            crate::model::VersionSource::Direct { .. }
        ));
        assert!(matches!(
            manifest.games[1].source,
            crate::model::VersionSource::GithubReleases { .. }
        ));
    }

    #[test]
    fn fetch_remote_reads_http_manifest() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let body = BUNDLED_MANIFEST.to_string();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        });

        let url = format!("http://127.0.0.1:{port}/manifest.json");
        let manifest = tauri::async_runtime::block_on(fetch_remote(&url, None, None)).unwrap();
        assert!(manifest.games.iter().any(|g| g.id == "dorm-monopoly"));
        server.join().unwrap();
    }
}
