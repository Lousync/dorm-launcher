import { useLauncher } from "../store";

export function Rail() {
  const view = useLauncher((s) => s.view);
  const setView = useLauncher((s) => s.setView);
  const appInfo = useLauncher((s) => s.appInfo);
  const activeCount = useLauncher(
    (s) => Object.values(s.tasks).filter((t) => !t.finished).length,
  );

  return (
    <aside className="rail">
      <div className="logo">🎲</div>
      <button
        className={"nav" + (view === "library" || view === "detail" ? " active" : "")}
        onClick={() => setView("library")}
      >
        <span className="ico">▦</span>游戏库
      </button>
      <button
        className={"nav" + (view === "downloads" ? " active" : "")}
        onClick={() => setView("downloads")}
      >
        <span className="ico">
          ⤓
          {activeCount > 0 && <span className="nav-badge">{activeCount}</span>}
        </span>
        下载
      </button>
      <button
        className={"nav" + (view === "settings" ? " active" : "")}
        onClick={() => setView("settings")}
      >
        <span className="ico">⚙</span>设置
      </button>
      <div className="spacer" />
      <div className="ver">宿舍游戏库 v{appInfo?.version ?? "…"}</div>
    </aside>
  );
}
