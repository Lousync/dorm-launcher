# 宿舍游戏库

宿舍主题游戏的**下载 / 更新 / 启动**管理器（启动器）。首款接管的游戏是
《[宿舍大富翁](https://github.com/Lousync/dorm-monopoly)》，后续新游戏只需在清单里登记一条即可被本管理器接管。

## 功能特性

- **游戏库**：卡片网格、三态（未安装 / 已是最新 / 可更新）、搜索、详情页（概览 / 版本历史 / 截图）。
- **下载管理**（Steam 式统一列表）：队列 + 并发上限、进度 / 速率 / 取消 / 重试、悬浮下载条、侧栏徽标。
- **安装**：流式下载、SHA-256 校验、zip 安全解压（防路径穿越 / 禁符号链接 / 自动剥壳）、多版本并存。
- **启动 / 更新 / 回退 / 卸载**：以版本目录为工作目录拉起游戏；一键切换版本；删除版本并维护状态。
- **清单（catalog）**：仓库根 `catalog/manifest.json` 单一来源（内置兜底 + 远程托管），远程 → 缓存 → 内置三级回退。
- **GitHub Releases 动态源**：按 repo + 资产正则自动解析最新带资产的 release。
- **启动器自更新**：`tauri-plugin-updater` + 签名 + `latest.json`；设置页手动检查 + 启动后台静默检查。
- **网络适配**：代理（下载 / 清单 / GitHub API / 自更新）与 GitHub 加速镜像。

## 技术栈

- **外壳**：Tauri 2（Rust 后端 + 系统 WebView2）
- **后端**：Rust
- **前端**：React 19 + TypeScript + Vite
- **分发**：GitHub Releases（游戏包）+ 权威 `manifest.json`（清单）

设计与协议细节见 `.claude/plans/宿舍游戏库启动器-总计划.md`。

## 快速开始

```powershell
git clone https://github.com/Lousync/dorm-launcher.git
cd dorm-launcher
start.bat            # 自动装依赖并跑开发版（或 npm run tauri dev）
```

打包安装包：`start.bat build`（或 `npm run tauri build`），产物在 `src-tauri/target/release/bundle/`。

## 环境要求（Windows）

- Node.js ≥ 20（本机开发用 24.x）
- Rust 稳定版（本机 1.99）
- WebView2 运行时（Win11 自带；Win10 由安装包引导）
- 可选：Visual Studio C++ Build Tools（Rust `msvc` 工具链）

## 常用命令

```powershell
npm install            # 安装前端依赖（首次）
npm run tauri dev      # 开发模式（前端热更新 + Tauri 窗口）
npm run tauri build    # 打包发布（NSIS / MSI）
npm run build          # 仅构建前端（tsc + vite）
cargo check            # 仅在 src-tauri/ 下：后端类型检查
cargo fmt              # 后端格式化
```

## 目录结构

```
index.html            前端入口
src/                  React 前端源码
  App.tsx             应用外壳（导航 + 视图切换）
  store.ts            zustand 状态（清单 / 配置 / 路由 / 下载任务 / 自更新）
  api.ts              后端命令的类型化封装
  types.ts            与后端 model 对应的类型
  ui.ts               格式化 / 打开目录 / 选择目录等工具
  components/         Rail / Library / GameCard / Detail / Downloads / DownloadBar
                      / Settings / Cover / Toast
  App.css             全局样式（CSS 变量主题）
catalog/manifest.json 权威清单（内置兜底 + 远程托管，单一来源）
prototypes/           界面原型（评审存档）与 preview PNG
src-tauri/            Rust 后端（Tauri）
  src/main.rs         可执行入口
  src/lib.rs          Tauri Builder + command / 插件注册
  src/commands.rs     IPC 命令面
  src/downloads.rs    下载管理器（队列 + 并发 + 事件）
  src/install.rs      下载 / 校验 / 解压流水线
  src/catalog.rs      清单解析（内置 / 缓存 / 远程 + 动态源补水）
  src/github.rs       GitHub Releases API 动态解析
  src/model.rs        清单协议与视图模型
  src/config.rs       用户设置
  src/state.rs        本地安装状态
  src/paths.rs        目录规划
  src/error.rs        统一错误
  tauri.conf.json     应用配置（产品名 / 窗口 / 打包 / updater）
  Cargo.toml          后端依赖
.github/workflows/    release.yml（打 tag 自动构建签名发布）
start.bat             一键启动脚本（ASCII）
.claude/plans/        设计计划（本仓库）
```

## 里程碑

- [x] **M0** 脚手架：Tauri 2 + React TS + Vite，跑通构建
- [x] **M1** 清单加载 + 游戏库列表 + 详情页 + 设置页（按 `prototypes/ui-prototype.html` 落地）
- [x] **M2** 下载（进度 / 取消）/ SHA-256 校验 / zip 安全解压 / 安装状态登记
- [x] **M3** 启动游戏 / 更新 / 多版本切换（回退）/ 卸载
- [x] **M4** 远程清单（含缓存与离线回退）
- [x] **M5** GitHub Releases 动态源（按 repo 自动解析最新带资产的 release）
- [x] **M6** 启动器自更新（签名 + `latest.json`）
- [ ] **M7** 收尾与 CI 打包

## 下载管理（Steam 式统一列表）

下载 / 安装走**统一的队列管理**（`src-tauri/src/downloads.rs`），而不是弹窗：

- 左栏「**下载**」是统一的**任务列表**：进行中 / 排队中 / 已完成 / 失败 / 已取消 一屏可见，
  每行带进度条、速率、取消 / 重试 / 打开目录。
- **并发上限**：同时最多 `并发下载数`（设置页，1~8）个任务，其余**排队**；完成一个自动启动下一个。
- 右下角有**悬浮下载条**（有进行中任务且不在下载页时显示），点它直达下载页；左栏徽标显示进行中数量。
- 重复点同一版本不会重复排队；「清除已完成」清掉已结束的条目。

## 启动器自更新

用 `tauri-plugin-updater`：检查 GitHub Releases 上的 `latest.json`，比对版本后下载**签名**的安装包并重启。

- **配置**（`src-tauri/tauri.conf.json` → `plugins.updater`）：`pubkey`（更新签名公钥）与 `endpoints`
  （默认 `https://github.com/Lousync/dorm-launcher/releases/latest/download/latest.json`）。
  **仓库改名 / 换账号后要同步改 endpoint**。
- **签名密钥**：公钥已嵌入配置；**私钥绝不能入库**（`.gitignore` 已忽略 `*.key`）。
  私钥 + 口令放仓库 Secrets：`TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
  （本密钥无口令，可留空）。
- **发布流程**：改 `tauri.conf.json` 的 `version` → 打 tag（`vX.Y.Z`）并 push →
  `.github/workflows/release.yml` 在 `windows-latest` 上构建、签名、生成 `latest.json`、建**草稿** Release；
  在 GitHub 上把草稿发布后，旧版启动器下次检查即可一键更新。
- **界面**：设置页「检查更新」；开启「启动器自动更新」时启动后台静默检查，有新版会提示，可在设置页一键更新。

> **国内网络提示**：GitHub 直连常被墙。设置页「下载镜像」默认填了 `https://gh-proxy.com/`，
> 会自动给 GitHub 链接套加速前缀；也可留空直连或换用其它镜像（如 `https://ghfast.top/`）。
> 「代理」填 `http://127.0.0.1:7890` 之类，对**游戏下载、远程清单、GitHub API 解析、启动器自更新**
> 全部生效（走 `reqwest` / updater 的代理）；「下载镜像」只作用于游戏下载 / 清单里的 GitHub 链接。

## 游戏清单（catalog）

`catalog/manifest.json` 是**唯一权威清单**：它既被打包进程序当**离线兜底**，也被当作
**远程托管文件**（用同一份，避免分叉）。新增游戏只需在它的 `games` 数组里加一条，**不用改启动器代码**。

托管方式（二选一）：

1. **仓库原始文件（推荐）**：把 `catalog/manifest.json` 推到启动器仓库后，设置页「清单 URL」填
   `https://raw.githubusercontent.com/<你的账号>/<启动器仓库>/main/catalog/manifest.json`
   （下载镜像会自动套用）。
2. **GitHub Pages / 任意静态托管**：把该文件放到一个稳定 URL 即可。

启动器行为：**首次启动**（无缓存）或点「刷新清单」时拉远程；启动时若开着「自动检查更新」且填了清单 URL，
会**后台静默**刷新一次；拉取失败自动回退到**缓存 → 内置清单**，并在界面给出提示。

## 约定

工程约定（分支、提交、风格、测试）见仓库根 `AGENTS.md`。
