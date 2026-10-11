import { useEffect } from "react";
import { useLauncher } from "./store";
import { Rail } from "./components/Rail";
import { Library } from "./components/Library";
import { Downloads } from "./components/Downloads";
import { Detail } from "./components/Detail";
import { Settings } from "./components/Settings";
import { Toast } from "./components/Toast";
import { DownloadBar } from "./components/DownloadBar";
import "./App.css";

function App() {
  const bootstrap = useLauncher((s) => s.bootstrap);
  const view = useLauncher((s) => s.view);
  const loading = useLauncher((s) => s.loading);
  const catalog = useLauncher((s) => s.catalog);
  const error = useLauncher((s) => s.error);
  const clearError = useLauncher((s) => s.clearError);

  useEffect(() => {
    bootstrap();
  }, [bootstrap]);

  return (
    <div className="app">
      <Rail />
      <main className="main">
        {error && (
          <div className="banner err top" onClick={clearError}>
            {error}（点击关闭）
          </div>
        )}
        {loading && !catalog ? (
          <div className="loading">正在加载…</div>
        ) : view === "library" ? (
          <Library />
        ) : view === "downloads" ? (
          <Downloads />
        ) : view === "detail" ? (
          <Detail />
        ) : (
          <Settings />
        )}
      </main>
      <Toast />
      <DownloadBar />
    </div>
  );
}

export default App;
