# AGENTS.md — 宿舍游戏库启动器 · 工程约定

> 面向在此仓库工作的 AI 编码代理与人类开发者。**本文件是工程约定的单一来源**：
> 目录结构、命令、分支/提交规范、清单协议都以此为准。
> 更新：2026-10-10

## 一、项目

**宿舍游戏库**：宿舍主题游戏的**下载 / 更新 / 启动**管理器。Tauri 2（Rust 后端）+ React 19 + TS + Vite。
首款接管《宿舍大富翁》，后续游戏通过清单登记接入，**不改启动器代码**。

- 设计基线与协议：`.claude/plans/宿舍游戏库启动器-总计划.md`（**开工前先读**）
- 上游游戏仓库：`https://github.com/Lousync/dorm-monopoly`

## 二、目录

| 路径 | 内容 |
|---|---|
| `src/` | React 前端（`App.tsx` / `App.css` / `main.tsx`） |
| `src-tauri/src/lib.rs` | Tauri Builder、`invoke_handler`、所有 command |
| `src-tauri/src/main.rs` | 可执行入口（勿动） |
| `src-tauri/tauri.conf.json` | 产品名 / 窗口 / 打包配置 |
| `src-tauri/Cargo.toml` | 后端依赖 |
| `.claude/plans/` | 设计计划（随版本开发，合并后删） |

## 三、常用命令

```powershell
npm install          # 首次
npm run tauri dev    # 开发（热更新）
npm run build        # 前端构建（tsc + vite）
npm run tauri build  # 打包
cargo check          # 后端类型检查（在 src-tauri/ 下）
cargo fmt            # 后端格式化
cargo clippy         # 后端 lint（有则跑）
```

**改完必跑**：前端改 → `npm run build`；后端改 → `cargo check`（+ `cargo fmt`）。两者都改 → 都跑。

## 四、硬性约定

- **前后端边界**：所有系统能力（网络 / 文件 / 进程）只在 **Rust 侧**实现，前端只通过 `invoke` 调
  command + 监听事件；前端**不直接**碰文件系统 / 发外部 HTTP。
- **IPC 命名**：command 用 `snake_case`；事件用 `域:动作`（如 `task:progress`）。
- **进度类事件节流**：最多约 10 次/秒 emit，避免刷爆 WebView。
- **路径安全**：解压 zip 必须逐条校验目标路径落在目标目录内（防 zip-slip），禁用符号链接。
- **完整性**：任何游戏包**先校验 SHA-256 再解压**，校验失败必须中止并清理。
- **语言**：界面与文档用中文；代码标识符用英文。
- **风格**：Rust 走 `rustfmt`（4 空格）；前端 `prettier` 默认；**Rust 缩进不要用 Tab**。
- **不提交大文件与产物**：`node_modules/`、`dist/`、`src-tauri/target/`、游戏包一律 ignore。

## 五、分支 / 提交

- **不在 `main` 直接改**：新功能 `feature/<版本>`、修 bug `fix/<版本>`。
- **提交信息用中文**，说清「做了什么 + 为什么」，一条提交聚焦单一改动。
- **只在用户明确要求时 commit / push**（本仓库沿用上游习惯：改完可自动提交，但推送、合并需用户确认）。

## 六、清单协议（`manifest.json`）

- **权威 schema 与托管方式见 `.claude/plans/宿舍游戏库启动器-总计划.md` §四**，不在本文件复述。
- **唯一文件 = 仓库根 `catalog/manifest.json`**：它同时是**内置兜底**（`include_str!` 打进程序）
  与**远程托管源**。改游戏目录只动这一个文件，别另建第二份。
- 关键点：`schemaVersion` 版本化；`id` 仅 `[a-z0-9-]`；`version` 走 semver；`sha256` 必填；
  `source.type` 支持 `github-releases`（动态解析）与 `direct`（写死直链）。
- 拉取优先级：远程 → 缓存（`%APPDATA%\DormGameHub\catalog-cache.json`）→ 内置。
- **动态源**：`source.type = github-releases` 的游戏，在**刷新清单**时查 GitHub API
  (`/repos/{repo}/releases`)，挑出带匹配资产（`assetPattern`，命名组 `version`）的 release，按 semver 取最新；
  `sha256` 优先用资产 `digest`，缺失则回填清单写死的值，仍无则丢弃该版本。失败一律保留清单写死的版本兜底。

## 七、当前状态

- 里程碑进度见 `README.md`；**具体待办以计划文档与开发中的清单为准**，本文件不复述。
- 已完成：M0 脚手架（`tauri dev` / `build` 可跑）；M1 真实 UI（游戏库 / 详情 / 设置，均按
  `prototypes/ui-prototype.html` 落地）；M2 下载 / SHA-256 校验 / zip 安全解压 / 安装状态登记；
  M3 启动游戏（工作目录 = 版本目录）/ 多版本切换（回退）/ 卸载；
  M4 远程清单（远程 → 缓存 → 内置三级回退 + 启动后台自动刷新）；
  M5 GitHub Releases 动态源（刷新时查 API 解析最新带资产的 release）；
  **统一下载管理**（`downloads.rs`：队列 + 并发上限 + `task:progress`/`task:done` 事件驱动的列表页）；
  M6 启动器自更新（`tauri-plugin-updater` + 签名 + `latest.json`，工作流 `.github/workflows/release.yml`）。
- **下载调度**：`DownloadManager`（`downloads.rs`）是唯一入口 —— 入队 → 按 `config.concurrency`
  调度 → 逐个跑 `install::run`。别再绕过它直接跑安装（会丢掉排队 / 取消 / 事件）。
- **自更新密钥**：更新签名**私钥绝不能入库**（`.gitignore` 已忽略 `*.key`）；公钥在
  `tauri.conf.json` 的 `plugins.updater.pubkey`。改公钥 / 换仓库要同步改 `endpoints`。
- **国内网络**：下载与远程清单会对 GitHub 链接套「下载镜像」前缀（配置默认 `https://gh-proxy.com/`，
  可在设置页改 / 留空）；解压必须逐条校验路径（防 zip-slip）、禁符号链接 —— 已有单测钉住。
