import { useLauncher } from "../store";

/** 右下角悬浮的下载指示条（有进行中任务且不在「下载」页时显示）。 */
export function DownloadBar() {
  const view = useLauncher((s) => s.view);
  const setView = useLauncher((s) => s.setView);
  const tasks = useLauncher((s) => s.tasks);
  const games = useLauncher((s) => s.catalog?.games) ?? [];

  const active = Object.values(tasks).filter((t) => !t.finished);
  if (active.length === 0 || view === "downloads") return null;

  const t = active[0];
  const name = games.find((g) => g.id === t.gameId)?.name ?? t.gameId;
  const pct =
    t.phase === "download"
      ? t.total
        ? Math.round((t.received / t.total) * 100)
        : 0
      : t.phase === "queued"
        ? 0
        : 100;

  return (
    <button className="dl-bar" onClick={() => setView("downloads")}>
      <div className="dl-bar-row">
        <span className="dl-bar-title">下载中 {active.length}</span>
        <span className="dl-bar-pct">{pct}%</span>
      </div>
      <div className="dl-bar-sub">
        {name} v{t.version}
      </div>
      <div className="dl-bar-track">
        <i style={{ width: `${pct}%` }} />
      </div>
    </button>
  );
}
