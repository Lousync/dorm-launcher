import { useEffect, useState } from "react";
import { useLauncher } from "../store";
import { openFolder, pickDirectory } from "../ui";

export function Settings() {
  const config = useLauncher((s) => s.config);
  const appInfo = useLauncher((s) => s.appInfo);
  const appDataDir = useLauncher((s) => s.appDataDir);
  const defaultRoot = useLauncher((s) => s.defaultInstallRoot);
  const saveConfig = useLauncher((s) => s.saveConfig);
  const showToast = useLauncher((s) => s.showToast);
  const appUpdate = useLauncher((s) => s.appUpdate);
  const updateStatus = useLauncher((s) => s.updateStatus);
  const updateProgress = useLauncher((s) => s.updateProgress);
  const checkAppUpdate = useLauncher((s) => s.checkAppUpdate);
  const installAppUpdate = useLauncher((s) => s.installAppUpdate);

  const [catalogUrl, setCatalogUrl] = useState("");
  const [mirror, setMirror] = useState("");
  const [proxy, setProxy] = useState("");

  useEffect(() => setCatalogUrl(config?.catalogUrl ?? ""), [config?.catalogUrl]);
  useEffect(() => setMirror(config?.downloadMirror ?? ""), [config?.downloadMirror]);
  useEffect(() => setProxy(config?.proxy ?? ""), [config?.proxy]);

  if (!config) return null;

  const installRoot = config.installRoot || defaultRoot || "";

  const onPickRoot = async () => {
    const dir = await pickDirectory(installRoot);
    if (dir && (await saveConfig({ installRoot: dir }))) {
      showToast("安装根目录已更新");
    }
  };

  const commitCatalogUrl = async () => {
    const val = catalogUrl.trim();
    if (val !== (config.catalogUrl ?? "")) {
      if (await saveConfig({ catalogUrl: val || null }))
        showToast("清单地址已保存");
    }
  };

  const commitProxy = async () => {
    const val = proxy.trim();
    if (val !== (config.proxy ?? "")) {
      if (await saveConfig({ proxy: val || null })) showToast("代理已保存");
    }
  };

  const commitMirror = async () => {
    const val = mirror.trim();
    if (val !== (config.downloadMirror ?? "")) {
      if (await saveConfig({ downloadMirror: val || null }))
        showToast("下载镜像已保存");
    }
  };

  const onConcurrency = async (v: string) => {
    const n = Math.max(1, Math.min(8, parseInt(v, 10) || 1));
    if (n !== config.concurrency) await saveConfig({ concurrency: n });
  };

  const toggle = async (key: "autoCheckUpdate" | "launcherAutoUpdate") => {
    const patch: Partial<import("../types").AppConfig> = {};
    patch[key] = !config[key];
    await saveConfig(patch);
  };

  return (
    <section className="screen">
      <div className="page-head">
        <h2>设置</h2>
      </div>

      <div className="settings">
        <div className="group">
          <h3>常规</h3>
          <div className="gdesc">游戏文件与下载的存放位置。</div>
          <div className="field">
            <div className="label">
              安装根目录
              <span className="tip">默认 %LOCALAPPDATA%\DormGameHub</span>
            </div>
            <div className="ctl">
              <input className="input" value={installRoot} readOnly />
              <button onClick={onPickRoot}>选择…</button>
            </div>
          </div>
          <div className="field">
            <div className="label">
              并发下载数
              <span className="tip">同时下载的任务上限（1~8）</span>
            </div>
            <div className="ctl">
              <input
                className="input num"
                type="number"
                min={1}
                max={8}
                value={config.concurrency}
                onChange={(e) => onConcurrency(e.target.value)}
              />
            </div>
          </div>
        </div>

        <div className="group">
          <h3>更新源</h3>
          <div className="gdesc">清单地址与网络相关设置。</div>
          <div className="field">
            <div className="label">
              清单 URL
              <span className="tip">留空则只用内置清单</span>
            </div>
            <div className="ctl">
              <input
                className="input"
                value={catalogUrl}
                onChange={(e) => setCatalogUrl(e.target.value)}
                onBlur={commitCatalogUrl}
                placeholder="https://raw.githubusercontent.com/…/manifest.json"
              />
            </div>
          </div>
          <div className="field">
            <div className="label">
              下载镜像
              <span className="tip">GitHub 加速前缀，如 https://gh-proxy.com/</span>
            </div>
            <div className="ctl">
              <input
                className="input"
                value={mirror}
                onChange={(e) => setMirror(e.target.value)}
                onBlur={commitMirror}
                placeholder="https://gh-proxy.com/"
              />
            </div>
          </div>
          <div className="field">
            <div className="label">代理</div>
            <div className="ctl">
              <input
                className="input"
                value={proxy}
                onChange={(e) => setProxy(e.target.value)}
                onBlur={commitProxy}
                placeholder="http://127.0.0.1:7890"
              />
            </div>
          </div>
          <div className="field">
            <div className="label">自动检查更新</div>
            <div className="ctl">
              <span
                className={"switch" + (config.autoCheckUpdate ? " on" : "")}
                onClick={() => toggle("autoCheckUpdate")}
              />
            </div>
          </div>
          <div className="field">
            <div className="label">启动器自动更新</div>
            <div className="ctl">
              <span
                className={"switch" + (config.launcherAutoUpdate ? " on" : "")}
                onClick={() => toggle("launcherAutoUpdate")}
              />
            </div>
          </div>
        </div>

        <div className="group">
          <h3>关于</h3>
          <div className="field">
            <div className="label">启动器版本</div>
            <div className="ctl">
              <span className="muted">v{appInfo?.version ?? "…"}</span>
              <button onClick={() => checkAppUpdate(false)}>检查更新</button>
              <button onClick={() => openFolder(appDataDir)}>打开数据目录</button>
            </div>
          </div>
          {appUpdate && (
            <div className="field">
              <div className="label">新版本</div>
              <div className="ctl">
                <span>v{appUpdate.version} 可用</span>
                {updateStatus === "downloading" ? (
                  <span className="muted">下载中 {updateProgress}%</span>
                ) : updateStatus === "ready" ? (
                  <span className="muted">已安装，重启生效</span>
                ) : (
                  <button className="primary" onClick={() => installAppUpdate()}>
                    立即更新
                  </button>
                )}
              </div>
            </div>
          )}
        </div>
      </div>
    </section>
  );
}
