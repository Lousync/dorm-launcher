use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter};

use crate::catalog;
use crate::error::{AppError, AppResult};
use crate::model::{Game, GameVersion, TaskProgress};
use crate::paths;
use crate::state::{self, InstalledVersion};

const EMIT_INTERVAL: Duration = Duration::from_millis(120);

fn emit_progress(
    app: &AppHandle,
    task_id: &str,
    game_id: &str,
    phase: &str,
    received: u64,
    total: Option<u64>,
    speed_bps: f64,
) {
    let _ = app.emit(
        "task:progress",
        TaskProgress {
            task_id: task_id.to_string(),
            game_id: game_id.to_string(),
            phase: phase.to_string(),
            received,
            total,
            speed_bps,
        },
    );
}

/// 完整安装流程：下载 → 校验 SHA-256 → 解压到版本目录 → 登记状态。
/// 成功返回版本目录的绝对路径。
#[allow(clippy::too_many_arguments)]
pub async fn run(
    app: &AppHandle,
    task_id: &str,
    game: &Game,
    version: &GameVersion,
    root: &Path,
    cancel: Arc<AtomicBool>,
    proxy: Option<&str>,
    mirror: Option<&str>,
) -> AppResult<String> {
    let cache_dir = root.join("cache");
    std::fs::create_dir_all(&cache_dir)?;
    let part = cache_dir.join(format!("{}-{}.zip.part", game.id, version.version));

    // 1) 下载
    emit_progress(app, task_id, &game.id, "download", 0, version.size, 0.0);
    let downloaded = download(
        app,
        task_id,
        &game.id,
        &version.url,
        &part,
        version.size,
        &cancel,
        proxy,
        mirror,
    )
    .await?;
    if cancelled(&cancel) {
        let _ = std::fs::remove_file(&part);
        return Err(AppError::Other("已取消".into()));
    }

    // 2) 校验
    emit_progress(
        app,
        task_id,
        &game.id,
        "verify",
        downloaded,
        Some(downloaded),
        0.0,
    );
    let part_for_hash = part.clone();
    let actual = tauri::async_runtime::spawn_blocking(move || sha256_file(&part_for_hash))
        .await
        .map_err(|e| AppError::Other(format!("校验任务失败：{e}")))??;
    if !actual.eq_ignore_ascii_case(version.sha256.trim()) {
        let _ = std::fs::remove_file(&part);
        return Err(AppError::Other(format!(
            "SHA-256 校验失败：期望 {}，实际 {}",
            version.sha256, actual
        )));
    }

    // 3) 解压到临时目录
    emit_progress(app, task_id, &game.id, "extract", 0, None, 0.0);
    let versions_dir = paths::versions_dir(root, &game.id);
    std::fs::create_dir_all(&versions_dir)?;
    let tmp = versions_dir.join(format!("{}.tmp", version.version));
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    std::fs::create_dir_all(&tmp)?;

    let zip_path = part.clone();
    let tmp_for_extract = tmp.clone();
    let cancel_for_extract = cancel.clone();
    let extract_result = tauri::async_runtime::spawn_blocking(move || {
        extract_zip(&zip_path, &tmp_for_extract, &cancel_for_extract)
    })
    .await
    .map_err(|e| AppError::Other(format!("解压任务失败：{e}")))?;
    if let Err(e) = extract_result {
        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }

    // 4) 落位（先删旧目录，再改名，尽量原子）
    let final_dir = paths::version_dir(root, &game.id, &version.version);
    if final_dir.exists() {
        std::fs::remove_dir_all(&final_dir)?;
    }
    std::fs::rename(&tmp, &final_dir)?;
    let _ = std::fs::remove_file(&part);

    // 5) 登记状态
    record_install(&game.id, &version.version, &final_dir, &actual)?;

    Ok(final_dir.to_string_lossy().to_string())
}

fn cancelled(cancel: &AtomicBool) -> bool {
    cancel.load(Ordering::SeqCst)
}

#[allow(clippy::too_many_arguments)]
async fn download(
    app: &AppHandle,
    task_id: &str,
    game_id: &str,
    url: &str,
    dest: &Path,
    size_hint: Option<u64>,
    cancel: &AtomicBool,
    proxy: Option<&str>,
    mirror: Option<&str>,
) -> AppResult<u64> {
    let mut builder = reqwest::Client::builder()
        .user_agent(concat!("dorm-launcher/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(600));
    if let Some(p) = proxy.filter(|p| !p.trim().is_empty()) {
        builder = builder.proxy(reqwest::Proxy::all(p)?);
    }
    let client = builder.build()?;

    let target = catalog::apply_mirror(url, mirror);
    let resp = client.get(&target).send().await?.error_for_status()?;
    let total = resp.content_length().or(size_hint);

    let mut file = std::fs::File::create(dest)?;
    let mut received: u64 = 0;
    let started = Instant::now();
    let mut last_emit = Instant::now();
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        if cancelled(cancel) {
            return Err(AppError::Other("已取消".into()));
        }
        let chunk = chunk?;
        file.write_all(&chunk)?;
        received += chunk.len() as u64;
        if last_emit.elapsed() >= EMIT_INTERVAL {
            let secs = started.elapsed().as_secs_f64().max(0.001);
            emit_progress(
                app,
                task_id,
                game_id,
                "download",
                received,
                total,
                received as f64 / secs,
            );
            last_emit = Instant::now();
        }
    }
    file.flush()?;
    drop(file);

    let secs = started.elapsed().as_secs_f64().max(0.001);
    emit_progress(
        app,
        task_id,
        game_id,
        "download",
        received,
        total.or(Some(received)),
        received as f64 / secs,
    );
    Ok(received)
}

fn sha256_file(path: &Path) -> AppResult<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    Ok(out)
}

/// 安全解压：拒绝路径穿越与符号链接；若整包只有一个顶层目录则自动剥掉。
fn extract_zip(zip_path: &Path, dest: &Path, cancel: &AtomicBool) -> AppResult<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| AppError::Other(format!("打开压缩包失败：{e}")))?;

    let strip = common_root(&mut archive)?;

    for i in 0..archive.len() {
        if cancelled(cancel) {
            return Err(AppError::Other("已取消".into()));
        }
        let mut entry = archive
            .by_index(i)
            .map_err(|e| AppError::Other(format!("读取压缩条目失败：{e}")))?;

        let raw = match entry.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => {
                return Err(AppError::Other(format!(
                    "压缩包含非法路径：{}",
                    entry.name()
                )));
            }
        };

        let rel = match &strip {
            Some(prefix) => match raw.strip_prefix(prefix) {
                Ok(p) => p.to_path_buf(),
                Err(_) => continue,
            },
            None => raw,
        };
        if rel.as_os_str().is_empty() {
            continue;
        }

        // 跳过符号链接（unix mode 类型位 0o120000）。
        if let Some(mode) = entry.unix_mode()
            && mode & 0o170000 == 0o120000
        {
            continue;
        }

        let out = dest.join(&rel);
        if !out.starts_with(dest) {
            return Err(AppError::Other(format!("解压路径越界：{}", out.display())));
        }

        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut f = std::fs::File::create(&out)?;
            std::io::copy(&mut entry, &mut f)?;
        }
    }
    Ok(())
}

/// 判断压缩包是否所有条目都在同一个顶层目录下；是则返回该目录名（用于剥壳）。
fn common_root<R: Read + Seek>(archive: &mut zip::ZipArchive<R>) -> AppResult<Option<PathBuf>> {
    let mut root: Option<String> = None;
    let mut nested = false;

    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| AppError::Other(format!("读取压缩条目失败：{e}")))?;
        let name = entry.name().replace('\\', "/");
        let mut parts = name.splitn(2, '/');
        let first = parts.next().unwrap_or("");
        if first.is_empty() {
            return Ok(None);
        }
        if parts.next().is_some() {
            nested = true;
        }
        match &root {
            None => root = Some(first.to_string()),
            Some(r) if r == first => {}
            _ => return Ok(None),
        }
    }

    if nested {
        Ok(root.map(PathBuf::from))
    } else {
        Ok(None)
    }
}

fn record_install(id: &str, version: &str, dir: &Path, sha256: &str) -> AppResult<()> {
    let path = paths::state_path()?;
    let mut st = state::load(&path)?;
    let entry = st.games.entry(id.to_string()).or_default();

    if !entry.installed.iter().any(|v| v.version == version) {
        entry.installed.push(InstalledVersion {
            version: version.to_string(),
            path: dir.to_string_lossy().to_string(),
            installed_at: catalog::now_secs(),
            sha256: Some(sha256.to_string()),
        });
    }
    entry.current_version = Some(version.to_string());

    state::save(&path, &st)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::write::SimpleFileOptions;

    fn make_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zw = zip::ZipWriter::new(file);
        let opts = SimpleFileOptions::default();
        for (name, data) in entries {
            zw.start_file(*name, opts).unwrap();
            zw.write_all(data).unwrap();
        }
        zw.finish().unwrap();
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn sha256_matches_known_value() {
        let dir = temp_dir("dorm-launcher-sha");
        let f = dir.join("a.txt");
        std::fs::write(&f, b"abc").unwrap();
        assert_eq!(
            sha256_file(&f).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_strips_single_root_dir() {
        let dir = temp_dir("dorm-launcher-zip-strip");
        let zip_path = dir.join("pkg.zip");
        make_zip(
            &zip_path,
            &[("game/app.exe", b"BIN"), ("game/readme.txt", b"HI")],
        );
        let dest = dir.join("out");
        std::fs::create_dir_all(&dest).unwrap();
        extract_zip(&zip_path, &dest, &AtomicBool::new(false)).unwrap();
        assert!(dest.join("app.exe").exists());
        assert!(dest.join("readme.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_keeps_files_at_root() {
        let dir = temp_dir("dorm-launcher-zip-nostrip");
        let zip_path = dir.join("pkg.zip");
        make_zip(
            &zip_path,
            &[("DormMonopoly.exe", b"BIN"), ("LICENSE", b"MIT")],
        );
        let dest = dir.join("out");
        std::fs::create_dir_all(&dest).unwrap();
        extract_zip(&zip_path, &dest, &AtomicBool::new(false)).unwrap();
        assert!(dest.join("DormMonopoly.exe").exists());
        assert!(dest.join("LICENSE").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn extract_rejects_path_traversal() {
        let dir = temp_dir("dorm-launcher-zip-escape");
        let zip_path = dir.join("pkg.zip");
        make_zip(&zip_path, &[("../evil.txt", b"X"), ("ok.txt", b"Y")]);
        let dest = dir.join("out");
        std::fs::create_dir_all(&dest).unwrap();
        let res = extract_zip(&zip_path, &dest, &AtomicBool::new(false));
        assert!(res.is_err(), "含路径穿越的压缩包应被拒绝");
        assert!(!dir.join("evil.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
