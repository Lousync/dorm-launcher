import { useMemo, useState } from "react";
import { useLauncher } from "../store";
import { formatTime } from "../ui";
import { GameCard } from "./GameCard";

export function Library() {
  const catalog = useLauncher((s) => s.catalog);
  const config = useLauncher((s) => s.config);
  const refreshing = useLauncher((s) => s.refreshing);
  const refreshCatalog = useLauncher((s) => s.refreshCatalog);
  const [q, setQ] = useState("");

  const games = catalog?.games ?? [];
  const filtered = useMemo(() => {
    const key = q.trim().toLowerCase();
    if (!key) return games;
    return games.filter(
      (g) =>
        g.name.toLowerCase().includes(key) ||
        (g.summary ?? "").toLowerCase().includes(key),
    );
  }, [games, q]);

  const installedCount = games.filter((g) => g.status !== "notInstalled").length;
  const remoteFailed = catalog?.source === "bundled" && !!config?.catalogUrl?.trim();

  return (
    <section className="screen">
      <div className="page-head">
        <h2>游戏库</h2>
        <span className="count">共 {games.length} 款 · 已安装 {installedCount} 款</span>
        <span className="spacer" />
        <div className="search">
          <span>🔍</span>
          <input
            value={q}
            onChange={(e) => setQ(e.target.value)}
            placeholder="搜索游戏…"
          />
        </div>
        <button onClick={() => refreshCatalog()} disabled={refreshing}>
          {refreshing ? "刷新中…" : "↻ 刷新清单"}
        </button>
      </div>

      {remoteFailed && (
        <div className="banner warn">
          ⚠ 远程清单拉取失败，正在使用内置清单（可能不是最新）。
        </div>
      )}
      {catalog?.source === "cache" && (
        <div className="banner info">
          使用缓存清单（{formatTime(catalog.fetchedAt)}）。
        </div>
      )}

      {games.length === 0 ? (
        <div className="empty">
          <div className="big">📦</div>
          <h3>游戏库还是空的</h3>
          <p>清单里暂时没有可用游戏。</p>
          <button className="primary" onClick={() => refreshCatalog()}>
            ↻ 刷新清单
          </button>
        </div>
      ) : filtered.length === 0 ? (
        <div className="empty">
          <div className="big">🔍</div>
          <h3>没有匹配的游戏</h3>
          <p>换个关键词试试。</p>
        </div>
      ) : (
        <div className="grid">
          {filtered.map((g) => (
            <GameCard key={g.id} game={g} />
          ))}
          <div className="card ghostcard">
            <div className="big">＋</div>
            新游戏接入后自动出现在这里
          </div>
        </div>
      )}
    </section>
  );
}
