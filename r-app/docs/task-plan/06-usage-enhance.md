# 06 — WP6 用量统计增强

> 关联：[TASKS.md](./TASKS.md) · [PRD-2.md](./PRD-2.md)
> 所属层：后端（Rust/Tauri）+ 前端
> 前置：WP5（5.1 Token 单位工具 / 5.2 稳定日期工具）

## 目标

修复"首次打开用量统计卡死主进程"（R2，同步命令在主线程做重 IO），并为用量统计页新增时间段筛选器（R3，默认今日）；用量总量卡片接入 Token 单位辅助小字（R4 复用）。

## 关键文件/落点

- 后端命令：`src-tauri/src/commands/usage.rs`（`sync_session_usage`）
- 后端同步：`src-tauri/src/modules/usage_local/mod.rs`（`sync_all`，**逻辑不动**）
- 状态：`src/state.rs`（`db_pool: DbPool`，r2d2 可克隆）
- 前端页面：`src/pages/Statistics/_components/UsagePanel.tsx`
- 前端服务：`src/services/modules/usage.ts`（`getSummary/getByModel/getByDay` 已支持 `start/end/appType`）
- 工具：WP5 的 `formatTokenCompact` 与稳定日期工具

## 任务拆解

- **6.1** `sync_session_usage` 异步化：
  - 改签名为 `pub async fn sync_session_usage(state: State<'_, AppState>) -> AppResult<UsageSyncResult>`。
  - 进入闭包前 `let pool = state.db_pool.clone();`，用 `tauri::async_runtime::spawn_blocking(move || { let conn = pool.get()?; Ok(usage_local::sync_all(&conn)) }).await`（错误归一为 `AppError`）。
  - 解析/去重/取差逻辑保持不变，仅迁移执行线程，结果一致。
- **6.2** 用量时间筛选器（前端）：
  - `UsagePanel` 新增范围状态（今日/近 7 天/近 30 天/全部），**默认今日**；用 WP5 稳定日期工具算 `start/end`（转 `YYYY-MM-DD`），并入三个查询的入参与 queryKey。
  - 三个查询（summary/byModel/byDay）随筛选刷新；同步 mutation 完成后 `invalidateQueries(["usage"])` 照旧。
  - 总量卡片（输入/输出/缓存 Token）主值保留精确，附 `formatTokenCompact` 辅助小字。

## 数据契约（无新增后端契约）

- 复用第一轮：`get_usage_summary/get_usage_by_model/get_usage_by_day(start?, end?, app_type?)`，`start/end` 为 `YYYY-MM-DD`。
- 仅 `sync_session_usage` 的执行模型由同步改异步，返回结构 `UsageSyncResult` 不变。

## 验收标准

- 首次进入"用量统计"Tab 界面**不卡死**，同步在后台进行，进行态可见（`sync.isPending`），完成后数据自动刷新。
- 时间筛选默认"今日"；切换后总量卡片/按模型/按天三处口径一致地随之过滤。
- 本机无对应目录/文件时优雅返回空、不报错崩溃（延续第一轮）。
- 总量卡片大数值带辅助单位小字。

## 测试点

- 后端：`usage_local` 既有解析/去重/取差测试保持通过（`spawn_blocking` 不改变结果，不新增断言）。
- 前端（vitest）：UsagePanel 默认以"今日"对应 `start/end` 发起查询；切换范围改变查询入参（mock `usageApi` 校验调用参数）；空数据态渲染。
