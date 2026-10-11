use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::{AppHandle, Emitter, Manager};

use crate::catalog;
use crate::config;
use crate::error::{AppError, AppResult};
use crate::model::{TaskDone, TaskProgress};
use crate::paths;

/// 一个待下载 / 安装的任务。
#[derive(Debug, Clone)]
pub struct Job {
    pub task_id: String,
    pub game_id: String,
    pub version: String,
}

/// 下载管理器：维护「排队 → 运行中」的队列，按并发上限调度。
#[derive(Default)]
pub struct DownloadManager {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    queue: VecDeque<Job>,
    running: HashSet<String>,
    cancels: HashMap<String, Arc<AtomicBool>>,
    /// 排队或运行中的 task_id（去重用）。
    known: HashSet<String>,
}

impl DownloadManager {
    /// 入队；若已在队列 / 运行中返回 false。
    fn enqueue(&self, job: Job) -> bool {
        let mut inner = self.inner.lock().expect("download manager poisoned");
        if !inner.known.insert(job.task_id.clone()) {
            return false;
        }
        inner.queue.push_back(job);
        true
    }

    /// 在并发上限内取出下一个待启动任务，并登记取消信号。
    fn start_next(&self, concurrency: usize) -> Option<(Job, Arc<AtomicBool>)> {
        let mut inner = self.inner.lock().expect("download manager poisoned");
        if inner.running.len() >= concurrency {
            return None;
        }
        let job = inner.queue.pop_front()?;
        inner.running.insert(job.task_id.clone());
        let flag = Arc::new(AtomicBool::new(false));
        inner.cancels.insert(job.task_id.clone(), flag.clone());
        Some((job, flag))
    }

    fn finish(&self, task_id: &str) {
        let mut inner = self.inner.lock().expect("download manager poisoned");
        inner.running.remove(task_id);
        inner.cancels.remove(task_id);
        inner.known.remove(task_id);
    }

    fn cancel(&self, task_id: &str) -> CancelOutcome {
        let mut inner = self.inner.lock().expect("download manager poisoned");
        if let Some(flag) = inner.cancels.get(task_id) {
            flag.store(true, Ordering::SeqCst);
            return CancelOutcome::Cancelling;
        }
        if let Some(pos) = inner.queue.iter().position(|j| j.task_id == task_id) {
            inner.queue.remove(pos);
            inner.known.remove(task_id);
            return CancelOutcome::Dequeued;
        }
        CancelOutcome::NotFound
    }
}

enum CancelOutcome {
    Cancelling,
    Dequeued,
    NotFound,
}

impl CancelOutcome {
    fn as_str(&self) -> &'static str {
        match self {
            CancelOutcome::Cancelling => "cancelling",
            CancelOutcome::Dequeued => "dequeued",
            CancelOutcome::NotFound => "notFound",
        }
    }
}

/// 加入下载队列并立即开始调度；返回 `taskId`。
pub fn enqueue(app: &AppHandle, game_id: String, version: String) -> String {
    let task_id = format!("{game_id}@{version}");
    let job = Job {
        task_id: task_id.clone(),
        game_id: game_id.clone(),
        version,
    };
    if app.state::<DownloadManager>().enqueue(job) {
        emit_progress(app, &task_id, &game_id, "queued", 0, None, 0.0);
        pump(app);
    }
    task_id
}

/// 取消任务（运行中 → 置取消位；排队中 → 直接出队）；返回处置结果。
pub fn cancel(app: &AppHandle, task_id: &str) -> String {
    let outcome = app.state::<DownloadManager>().cancel(task_id);
    if let CancelOutcome::Dequeued = outcome {
        let game_id = task_id.split('@').next().unwrap_or(task_id);
        emit_done(app, task_id, game_id, false, Some("已取消".into()), None);
    }
    outcome.as_str().to_string()
}

/// 按并发上限把队列里的任务尽量启动起来。
fn pump(app: &AppHandle) {
    let concurrency = config::load(&paths::config_path().unwrap_or_default())
        .map(|c| c.concurrency.max(1) as usize)
        .unwrap_or(2);

    while let Some((job, flag)) = app.state::<DownloadManager>().start_next(concurrency) {
        let app2 = app.clone();
        tauri::async_runtime::spawn(async move {
            run_job(app2, job, flag).await;
        });
    }
}

async fn run_job(app: AppHandle, job: Job, cancel: Arc<AtomicBool>) {
    let result = run_inner(&app, &job, cancel).await;
    let (ok, error, path) = match &result {
        Ok(path) => (true, None, Some(path.clone())),
        Err(e) => (false, Some(e.to_string()), None),
    };
    emit_done(&app, &job.task_id, &job.game_id, ok, error, path);
    app.state::<DownloadManager>().finish(&job.task_id);
    pump(&app);
}

async fn run_inner(app: &AppHandle, job: &Job, cancel: Arc<AtomicBool>) -> AppResult<String> {
    let cfg = config::load(&paths::config_path()?)?;
    let root = paths::install_root(&cfg)?;
    let (manifest, _source, _) = catalog::resolve(&cfg, false).await?;

    let game = manifest
        .games
        .iter()
        .find(|g| g.id == job.game_id)
        .ok_or_else(|| AppError::Manifest(format!("清单里没有游戏 {}", job.game_id)))?;
    let version = game
        .versions
        .iter()
        .find(|v| v.version == job.version)
        .cloned()
        .or_else(|| game.latest.clone().filter(|v| v.version == job.version))
        .ok_or_else(|| {
            AppError::Manifest(format!("游戏 {} 没有版本 {}", job.game_id, job.version))
        })?;

    crate::install::run(
        app,
        &job.task_id,
        game,
        &version,
        &root,
        cancel,
        cfg.proxy.as_deref(),
        cfg.download_mirror.as_deref(),
    )
    .await
}

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

fn emit_done(
    app: &AppHandle,
    task_id: &str,
    game_id: &str,
    ok: bool,
    error: Option<String>,
    path: Option<String>,
) {
    let _ = app.emit(
        "task:done",
        TaskDone {
            task_id: task_id.to_string(),
            game_id: game_id.to_string(),
            ok,
            error,
            path,
        },
    );
}
