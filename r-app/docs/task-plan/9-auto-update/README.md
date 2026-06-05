# 阶段 9：自动更新

> 任务总线：[../../TASKS.md](../../TASKS.md) ｜ 规划索引：[../README.md](../README.md)
> 起点参考：旧版 ccNexus 源码 `docs/origin/ccNexus/`（Go + Wails，仅参考不复用）

## 阶段目标

接入 `tauri-plugin-updater` + `process`，实现检查/下载（进度）/安装、跳过版本、自动检查间隔配置，以及前端更新红点/进度/跳过交互。交付 **里程碑 M5 完整前端** 的更新能力。

> 待确认：更新服务器 `endpoints` 与签名 `pubkey` 暂留空（待定分发渠道），本阶段先完成插件接入、配置占位与构件产出的结构性工作。

## 前置依赖

- 阶段 0（P0-6 注册中心）；
- 阶段 4（P4-1 配置：更新设置持久化）。

## 任务清单

### P9-1 更新插件接入
- 所属层：Rust
- 文件：`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json`、`src-tauri/capabilities/default.json`、`lib.rs`
- 实现要点：`pnpm tauri add updater` 与 `process`；`tauri.conf.json` 配置 `createUpdaterArtifacts` 与 updater `pubkey`/`endpoints`；capabilities 增加 updater/process 权限。生成签名密钥流程记录在文档。**注：更新服务器 `endpoints` 与签名 `pubkey` 暂留空，待后续确定分发渠道再填入；本任务先完成插件接入、配置占位与构件产出的结构性工作。**
- 前置：P0-6
- 验收：`tauri build` 产出更新构件；插件初始化无错误。
- PRD Story：71, 75

### P9-2 更新命令与设置
- 所属层：Rust
- 文件：`src-tauri/src/commands/update.rs`、`src-tauri/src/modules/storage/config_repo.rs`（update 设置）
- 实现要点：`check_for_updates`、`download_and_install`（用 Channel 推送下载进度 `update-progress`）、`get_update_settings`、`set_update_settings(autoCheck, checkInterval)`、`skip_version(version)`。检查间隔为 0 时停止自动检查。参考旧版 `updater.js`。
- 前置：P9-1, P4-1
- 验收：可检查到更新、下载有进度、可跳过版本、设置持久化。
- PRD Story：71, 72, 74, 75

### P9-3 更新前端（红点/进度/跳过）
- 所属层：React
- 文件：`src/components/business/UpdateBadge.tsx`（导航红点，跨页面）、`src/pages/Settings/_components/UpdateDialog.tsx`、`src/pages/Settings/_components/DownloadProgress.tsx`、`src/hooks/useUpdate.ts`、`src/pages/Settings/index.tsx`
- 实现要点：启动时按 `checkInterval` 决定是否检查；有新版本时侧边栏/导航显示红点 Badge；UpdateDialog 展示版本与变更，提供下载（监听 `update-progress` Channel 显示进度条）、跳过版本、稍后；设置页配置自动检查与间隔。
- 前置：P9-2, P0-8
- 验收：有更新显示红点，可下载并显示进度，可跳过版本，间隔可配置。
- PRD Story：71, 72, 73, 74, 75

## 参考材料（origin）

> 路径相对 `docs/origin/ccNexus/`

| 关注点 | 文件 | 说明 |
|--------|------|------|
| 更新核心 / 版本比较 | `internal/updater/{updater,version}.go` | 检查/跳过/间隔逻辑 |
| 下载 / 进度 | `internal/updater/downloader.go` | 下载进度回调 → 对标 Tauri Channel |
| 发布源 | `internal/updater/github.go` | release 拉取（本项目用 tauri updater endpoints 替代） |
| 安装应用 | `internal/updater/{apply_windows,apply_other}.go` | 安装流程（Tauri 插件接管） |
| 更新前端交互 | `cmd/desktop/frontend/src/modules/updater.js` | 红点 / 进度 / 跳过 UX |
| 构件产出 | `.github/workflows/build.yml` | CI 产物对标 `createUpdaterArtifacts` |

## 完成判据（里程碑 M5 之一）

- `tauri build` 产出更新构件，插件初始化无错误（endpoints/pubkey 可暂空）；
- 检查/下载（进度）/跳过版本/间隔配置可用，红点提示正确。
