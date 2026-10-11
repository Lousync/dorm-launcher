import { useState } from "react";
import { useLauncher } from "../store";
import {
  formatDate,
  formatSize,
  openFolder,
  STATUS_CLASS,
  STATUS_LABEL,
} from "../ui";
import { Cover } from "./Cover";

type Tab = "overview" | "versions" | "shots";

export function Detail() {
  const selectedId = useLauncher((s) => s.selectedId);
  const games = useLauncher((s) => s.catalog?.games) ?? [];
  const setView = useLauncher((s) => s.setView);
  const showToast = useLauncher((s) => s.showToast);
  const startInstall = useLauncher((s) => s.startInstall);
  const launchGame = useLauncher((s) => s.launchGame);
  const setCurrentVersion = useLauncher((s) => s.setCurrentVersion);
  const uninstallGame = useLauncher((s) => s.uninstallGame);
  const refreshCatalog = useLauncher((s) => s.refreshCatalog);
  const refreshing = useLauncher((s) => s.refreshing);
  const config = useLauncher((s) => s.config);
  const defaultRoot = useLauncher((s) => s.defaultInstallRoot);
  const [tab, setTab] = useState<Tab>("overview");

  const game = games.find((g) => g.id === selectedId);

  if (!game) {
    return (
      <section className="screen">
        <div className="page-head">
          <button onClick={() => setView("library")}>← 返回</button>
        </div>
        <div className="empty">
          <div className="big">❓</div>
          <h3>找不到这款游戏</h3>
        </div>
      </section>
    );
  }

  const primaryLabel =
    game.status === "notInstalled"
      ? "安装"
      : game.status === "updateAvailable"
        ? `更新到 ${game.latestVersion}`
        : "启动";

  const onOpenDir = async () => {
    const target = game.installPath || config?.installRoot || defaultRoot;
    const ok = await openFolder(target);
    if (!ok) showToast("打开目录失败");
  };

  return (
    <section className="screen">
      <div className="hero">
        <Cover src={game.cover || game.screenshots[0]} emoji="🎲" />
        <div className="veil" />
        <button className="btn back" onClick={() => setView("library")}>
          ← 返回
        </button>
        <div className="hero-inner">
          <h1>
            {game.name}{" "}
            <span className={`badge ${STATUS_CLASS[game.status]}`}>
              {STATUS_LABEL[game.status]}
            </span>
          </h1>
          <div className="desc">{game.description || game.summary}</div>
          <div className="detail-actions">
            <button
              className="primary"
              onClick={() => {
                if (game.status === "upToDate") {
                  launchGame(game.id);
                } else if (game.latestVersion) {
                  startInstall(game.id, game.latestVersion);
                }
              }}
            >
              {primaryLabel}
            </button>
            <button onClick={onOpenDir}>打开安装目录</button>
            {game.installedVersion && (
              <button
                className="danger"
                onClick={() =>
                  uninstallGame(game.id, game.installedVersion as string)
                }
              >
                卸载当前版本
              </button>
            )}
          </div>
        </div>
      </div>

      <div className="tabs">
        <button
          className={tab === "overview" ? "active" : ""}
          onClick={() => setTab("overview")}
        >
          概览
        </button>
        <button
          className={tab === "versions" ? "active" : ""}
          onClick={() => setTab("versions")}
        >
          版本历史
        </button>
        <button
          className={tab === "shots" ? "active" : ""}
          onClick={() => setTab("shots")}
        >
          截图
        </button>
      </div>

      {tab === "overview" && (
        <>
          <div className="info-grid">
            <div className="info">
              <div className="k">最新版本</div>
              <div className="v">{game.latestVersion ?? "—"}</div>
            </div>
            <div className="info">
              <div className="k">已安装</div>
              <div className="v">
                {game.installedVersion
                  ? game.installedVersion +
                    (game.status === "updateAvailable" ? "（旧）" : "")
                  : "未安装"}
              </div>
            </div>
            <div className="info">
              <div className="k">可执行文件</div>
              <div className="v">{game.executable}</div>
            </div>
            <div className="info">
              <div className="k">安装位置</div>
              <div className="v">{game.installPath ?? "—"}</div>
            </div>
          </div>
          {game.userDataNote && (
            <div className="foot-note">{game.userDataNote}</div>
          )}
        </>
      )}

      {tab === "versions" && (
        <div>
          <div className="tab-toolbar">
            <span className="muted small">
              共 {game.versions.length} 个版本
            </span>
            <span className="spacer" />
            <button
              onClick={() => refreshCatalog()}
              disabled={refreshing}
              title="重新拉取清单 / 动态解析最新 release"
            >
              {refreshing ? "刷新中…" : "↻ 刷新版本"}
            </button>
          </div>
          {game.versions.length === 0 && (
            <div className="muted">清单里没有可用的版本信息。</div>
          )}
          {game.versions.map((v) => {
            const isCurrent = v.version === game.installedVersion;
            const isInstalled = game.installedVersions.includes(v.version);
            return (
              <div key={v.version} className={"verrow" + (isCurrent ? " cur" : "")}>
                <span className="v">{v.version}</span>
                {isCurrent && <span className="pill">当前</span>}
                <span className="d">
                  {formatDate(v.publishedAt)} · {formatSize(v.size)}
                </span>
                <span className="verrow-action">
                  {isInstalled ? (
                    <>
                      {isCurrent ? (
                        <button disabled>当前版本</button>
                      ) : (
                        <button
                          className="primary"
                          onClick={() => setCurrentVersion(game.id, v.version)}
                        >
                          切换到此版本
                        </button>
                      )}
                      <button
                        className="danger"
                        onClick={() => uninstallGame(game.id, v.version)}
                      >
                        卸载
                      </button>
                    </>
                  ) : (
                    <button
                      className="primary"
                      onClick={() => startInstall(game.id, v.version)}
                    >
                      安装此版本
                    </button>
                  )}
                </span>
              </div>
            );
          })}
        </div>
      )}

      {tab === "shots" && (
        <div className="shots">
          {game.screenshots.length === 0 && (
            <div className="muted">没有截图。</div>
          )}
          {game.screenshots.map((s, i) => (
            <div className="shot" key={i}>
              <img src={s} alt="" onError={(e) => (e.currentTarget.style.display = "none")} />
            </div>
          ))}
        </div>
      )}
    </section>
  );
}
