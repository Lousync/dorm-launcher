import { useLauncher } from "../store";
import type { GameView } from "../types";
import { formatSize, STATUS_CLASS, STATUS_LABEL } from "../ui";
import { Cover } from "./Cover";

export function GameCard({ game }: { game: GameView }) {
  const openDetail = useLauncher((s) => s.openDetail);
  const showToast = useLauncher((s) => s.showToast);
  const startInstall = useLauncher((s) => s.startInstall);
  const launchGame = useLauncher((s) => s.launchGame);

  const primaryLabel =
    game.status === "notInstalled"
      ? "安装"
      : game.status === "updateAvailable"
        ? `更新到 ${game.latestVersion}`
        : "启动";

  const meta =
    game.status === "notInstalled"
      ? [`最新 ${game.latestVersion ?? "—"}`, formatSize(game.availableSize)]
      : [`已装 ${game.installedVersion ?? "—"}`, `最新 ${game.latestVersion ?? "—"}`];

  const onPrimary = () => {
    if (game.status === "upToDate") {
      launchGame(game.id);
      return;
    }
    if (!game.latestVersion) {
      showToast("清单里没有可安装的版本");
      return;
    }
    startInstall(game.id, game.latestVersion);
  };

  return (
    <div className="card">
      <div className="card-click" onClick={() => openDetail(game.id)}>
        <Cover src={game.cover} emoji="🎲">
          <span className={`badge ${STATUS_CLASS[game.status]}`}>
            {STATUS_LABEL[game.status]}
          </span>
        </Cover>
        <div className="body">
          <div className="title">{game.name}</div>
          <div className="sub">{game.summary}</div>
          <div className="meta">
            {meta.map((m, i) => (
              <span key={i}>{m}</span>
            ))}
          </div>
        </div>
      </div>
      <div className="actions">
        <button onClick={() => openDetail(game.id)}>详情</button>
        {game.status === "updateAvailable" && (
          <button onClick={() => launchGame(game.id)}>启动</button>
        )}
        <button className="primary" onClick={onPrimary}>
          {primaryLabel}
        </button>
      </div>
    </div>
  );
}
