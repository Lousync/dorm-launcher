use std::cmp::Ordering;

use regex::Regex;
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::model::GameVersion;

const API: &str = "https://api.github.com";

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
    /// GitHub 提供的资产摘要，形如 `sha256:<hex>`。
    #[serde(default)]
    digest: Option<String>,
}

/// 解析一个 GitHub 仓库的所有 release，挑出带匹配资产的版本，按 semver 降序返回。
/// `asset_pattern` 里的命名捕获组 `(?<version>…)` 用来取版本号；没有则从 tag 去掉前缀。
pub async fn resolve_releases(
    repo: &str,
    tag_prefix: Option<&str>,
    asset_pattern: &str,
    proxy: Option<&str>,
) -> AppResult<Vec<GameVersion>> {
    let re = Regex::new(asset_pattern)
        .map_err(|e| AppError::Manifest(format!("assetPattern 不是合法正则：{e}")))?;

    let mut builder = reqwest::Client::builder()
        .user_agent(concat!("dorm-launcher/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(20));
    if let Some(p) = proxy.filter(|p| !p.trim().is_empty()) {
        builder = builder.proxy(reqwest::Proxy::all(p)?);
    }
    let client = builder.build()?;

    let url = format!("{API}/repos/{repo}/releases?per_page=100");
    let releases: Vec<Release> = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(collect_versions(releases, &re, tag_prefix))
}

fn collect_versions(
    releases: Vec<Release>,
    re: &Regex,
    tag_prefix: Option<&str>,
) -> Vec<GameVersion> {
    let mut out: Vec<GameVersion> = Vec::new();

    for rel in releases {
        if rel.draft {
            continue;
        }
        let Some(asset) = rel.assets.iter().find(|a| re.is_match(&a.name)) else {
            continue;
        };

        let captures = re.captures(&asset.name);
        let version = captures
            .as_ref()
            .and_then(|c| c.name("version").map(|m| m.as_str().to_string()))
            .unwrap_or_else(|| match tag_prefix {
                Some(p) => rel
                    .tag_name
                    .strip_prefix(p)
                    .unwrap_or(&rel.tag_name)
                    .to_string(),
                None => rel.tag_name.clone(),
            });

        let sha256 = asset
            .digest
            .as_deref()
            .and_then(|d| d.strip_prefix("sha256:"))
            .unwrap_or("")
            .to_string();

        out.push(GameVersion {
            version,
            url: asset.browser_download_url.clone(),
            sha256,
            size: Some(asset.size),
            published_at: rel.published_at.clone(),
            notes: rel.body.clone(),
        });
    }

    out.sort_by(|a, b| compare_versions(&b.version, &a.version));
    out.dedup_by(|a, b| a.version == b.version);
    out
}

fn compare_versions(a: &str, b: &str) -> Ordering {
    match (semver::Version::parse(a), semver::Version::parse(b)) {
        (Ok(x), Ok(y)) => x.cmp(&y),
        _ => a.cmp(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, asset: &str, digest: Option<&str>, draft: bool) -> Release {
        Release {
            tag_name: tag.to_string(),
            draft,
            published_at: Some("2026-10-07T00:00:00Z".to_string()),
            body: None,
            assets: vec![Asset {
                name: asset.to_string(),
                browser_download_url: format!(
                    "https://github.com/u/r/releases/download/{tag}/{asset}"
                ),
                size: 100,
                digest: digest.map(|d| format!("sha256:{d}")),
            }],
        }
    }

    #[test]
    fn picks_highest_semver_with_matching_asset() {
        let re = Regex::new(r"^DormMonopoly_v?(?<version>.+?)\.zip$").unwrap();
        let releases = vec![
            release("v0.13.0", "DormMonopoly_v0.13.0.zip", Some("aa"), false),
            release("v0.13.1", "DormMonopoly_v0.13.1.zip", Some("bb"), false),
            release("v0.12.0", "DormMonopoly_v0.12.0.zip", Some("cc"), false),
            release("v0.14.0", "notes.txt", Some("dd"), false), // 资产不匹配 → 忽略
            release("v9.9.9", "DormMonopoly_v9.9.9.zip", Some("ee"), true), // draft → 忽略
        ];
        let out = collect_versions(releases, &re, Some("v"));
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].version, "0.13.1");
        assert_eq!(out[0].sha256, "bb");
        assert_eq!(out[2].version, "0.12.0");
    }

    #[test]
    fn version_from_tag_when_no_named_group() {
        let re = Regex::new(r"\.zip$").unwrap();
        let releases = vec![release("v1.2.3", "game.zip", Some("aa"), false)];
        let out = collect_versions(releases, &re, Some("v"));
        assert_eq!(out[0].version, "1.2.3");
    }

    #[test]
    fn missing_digest_yields_empty_sha() {
        let re = Regex::new(r"^x-(?<version>.+)\.zip$").unwrap();
        let releases = vec![release("v1.0.0", "x-1.0.0.zip", None, false)];
        let out = collect_versions(releases, &re, Some("v"));
        assert_eq!(out[0].sha256, "");
    }

    /// 联网实测（默认忽略）：`cargo test -- --ignored` 手动运行。
    #[test]
    #[ignore = "需要网络；本地手动跑 cargo test -- --ignored"]
    fn live_resolves_dorm_monopoly() {
        let out = tauri::async_runtime::block_on(resolve_releases(
            "Lousync/dorm-monopoly",
            Some("v"),
            r"^DormMonopoly_v?(?<version>.+?)\.zip$",
            None,
        ))
        .expect("应能拉取 GitHub release");
        assert!(
            out.iter().any(|v| v.version == "0.13.1"),
            "应解析出 0.13.1，实际：{:?}",
            out.iter().map(|v| &v.version).collect::<Vec<_>>()
        );
        assert!(out.iter().all(|v| v.sha256.len() == 64));
    }
}
