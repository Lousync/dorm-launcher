import { useLauncher, type TaskState } from "../store";
import { formatSize, openFolder } from "../ui";

const PHASE_TEXT: Record<string, string> = {
  queued: "排队中…",
  download: "下载中…",
  verify: "校验中…",
  extract: "解压中…",
  done: "完成",
};

function percent(t: TaskState): number {
  if (t.finished) return 100;
  if (t.phase === "download") {
    return t.total ? Math.min(100, Math.round((t.received / t.total) * 100)) : 0;
  }
  if (t.phase === "queued") return 0;
  return 100; // verify / extract
}

function statusLabel(t: TaskState): { text: string; cls: string } {
  if (t.finished) {
    if (t.ok) return { text: "已完成", cls: "ok" };
    if (t.error === "已取消") return { text: "已取消", cls: "none" };
    return { text: "失败", cls: "fail" };
  }
  if (t.phase === "queued") return { text: "排队中", cls: "inst" };
  return { text: "进行中", cls: "new" };
}

function metaText(t: TaskState): string {
  if (t.finished) {
    if (t.ok) return "已安装";
    return t.error ?? "已停止";
  }
  if (t.phase === "download") {
    const total = t.total ? ` / ${formatSize(t.total)}` : "";
    return `${formatSize(t.received)}${total} · ${formatSize(t.speed)}/s`;
  }
  return PHASE_TEXT[t.phase] ?? "";
}

export function Downloads() {
  const tasks = useLauncher((s) => s.tasks);
  const games = useLauncher((s) => s.catalog?.games) ?? [];
  const startInstall = useLauncher((s) => s.startInstall);
  const cancelInstall = useLauncher((s) => s.cancelInstall);
  const clearFinished = useLauncher((s) => s.clearFinished);

  const list = Object.values(tasks).sort(
    (a, b) => Number(a.finished) - Number(b.finished),
  );
  const activeCount = list.filter((t) => !t.finished).length;
  const finishedCount = list.length - activeCount;

  const nameOf = (id: string) => games.find((g) => g.id === id)?.name ?? id;

  return (
    <section className="screen">
      <div className="page-head">
        <h2>下载</h2>
        <span className="count">
          {activeCount} 个进行中 · {finishedCount} 个已结束
        </span>
        <span className="spacer" />
        <button onClick={clearFinished} disabled={finishedCount === 0}>
          清除已完成
        </button>
      </div>

      {list.length === 0 ? (
        <div className="empty">
          <div className="big">⤓</div>
          <h3>没有下载任务</h3>
          <p>在「游戏库」里点安装 / 更新，任务会出现在这里。</p>
        </div>
      ) : (
        <div className="task-list">
          {list.map((t) => {
            const st = statusLabel(t);
            return (
              <div className="task-row" key={t.taskId}>
                <div className="task-main">
                  <div className="task-title">
                    <span className="name">{nameOf(t.gameId)}</span>
                    <span className="muted small">v{t.version}</span>
                    <span className={`badge ${st.cls}`}>{st.text}</span>
                  </div>
                  <div className="bar">
                    <i
                      style={{ width: `${percent(t)}%` }}
                      className={t.finished && !t.ok ? "fail" : ""}
                    />
                  </div>
                  <div className="prog-meta">
                    <span>
                      {t.finished
                        ? st.text
                        : t.phase === "download"
                          ? `${percent(t)}%`
                          : PHASE_TEXT[t.phase]}
                    </span>
                    <span>{metaText(t)}</span>
                  </div>
                </div>
                <div className="task-actions">
                  {!t.finished ? (
                    <button
                      className="danger"
                      onClick={() => cancelInstall(t.taskId)}
                    >
                      取消
                    </button>
                  ) : t.ok ? (
                    <button onClick={() => openFolder(t.path)}>打开目录</button>
                  ) : (
                    <button
                      className="primary"
                      onClick={() => startInstall(t.gameId, t.version)}
                    >
                      重试
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}
