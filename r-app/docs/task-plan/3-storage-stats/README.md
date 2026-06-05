# 阶段 3：存储层与统计

> 任务总线：[../../TASKS.md](../../TASKS.md) ｜ 规划索引：[../README.md](../README.md)
> 起点参考：旧版 ccNexus 源码 `docs/origin/ccNexus/`（Go + Wails，仅参考不复用）

## 阶段目标

实现统计数据模型与仓库、四周期（今日/昨日/本周/本月）日期计算与聚合、事件驱动零延迟 + 2s 防抖落库、历史与月度归档，并提供统计命令与前端统计面板。交付 **里程碑 M3 数据闭环** 的统计部分。

## 前置依赖

- 阶段 0（P0-4 库表、P0-3 `AppState`、P0-5 设备 ID）；
- 事件推送依赖 P1-6（代理记录入口）。

## 任务清单

### P3-1 统计数据模型与仓库
- 所属层：Rust
- 文件：`src-tauri/src/models/stats.rs`、`src-tauri/src/modules/storage/stats_repo.rs`
- 实现要点：`DailyStat`（endpoint_name, date, requests, errors, input_tokens, output_tokens, device_id）、`PeriodStats`、`TrendCompare`、`CredentialUsage`。仓库提供 `upsert_daily_stat`（按 `UNIQUE(endpoint,date,device)` 累加）、`get_daily_stats(range)`、`get_period_aggregated`。
- 前置：P0-4
- 验收：累加 upsert、区间聚合单测通过。
- PRD Story：20, 21, 22, 23, 26, 30

### P3-2 四周期日期与聚合
- 所属层：Rust
- 文件：`src-tauri/src/modules/stats/periods.rs`、`aggregator.rs`
- 实现要点：`periods.rs` 计算今日/昨日/本周(周一起)/本月起始（chrono）；`aggregator.rs` 内存累计 `RecordRequest/RecordError/RecordTokens`，按端点与总量聚合；趋势对比（当前周期 vs 上一周期）。
- 前置：P3-1
- 验收：周期边界与趋势计算单测通过。
- PRD Story：16, 17, 18, 19, 25

### P3-3 防抖保存与事件推送
- 所属层：Rust
- 文件：`src-tauri/src/modules/stats/aggregator.rs`、`emitter.rs`
- 实现要点：统计变更后启动/重置 2s 防抖定时器批量落库（`scheduleSave`），降低写放大；落库后/变更时通过 `AppHandle.emit("stats-updated", payload)` 推送前端实现零延迟。代理转发成功/失败时调用记录接口。
- 前置：P3-2, P1-6
- 验收：连续多次记录在 2s 内只落库一次；前端能收到 `stats-updated` 事件。
- PRD Story：24, 性能（防抖）

### P3-4 历史与月度归档仓库
- 所属层：Rust
- 文件：`src-tauri/src/modules/storage/stats_repo.rs`
- 实现要点：`get_all_stats`、`get_archive_months`、`get_monthly_archive_data(month)`、`delete_monthly_stats(month)`（参数化）。
- 前置：P3-1
- 验收：归档查询与删除单测通过。
- PRD Story：78, 79, 80

### P3-5 统计与历史命令
- 所属层：Rust
- 文件：`src-tauri/src/commands/stats.rs`，注册到 `lib.rs`
- 实现要点：`get_period_stats(period)`、`get_trend(period)`、`get_endpoint_stats`、`get_archive_months`、`get_monthly_archive(month)`、`delete_monthly_stats(month)`。
- 前置：P3-2, P3-4
- 验收：前端可拉取四周期数据、趋势、历史并删除归档。
- PRD Story：16-25, 78-80

### P3-6 统计面板前端
- 所属层：React
- 文件：`src/pages/Statistics/index.tsx`、`src/pages/Statistics/_components/*`（`PeriodTabs`/`TrendBadge`/`EndpointStatsTable`；`StatCard` 跨页面复用，置 `src/components/business/StatCard.tsx`）、`src/hooks/useStats.ts`、`src/services/modules/stats.ts`(statsApi)
- 实现要点：用 shadcn Tabs 切换今日/昨日/本周/本月；StatCard 展示请求数/错误数/输入/输出 Token；EndpointStatsTable 按端点列出；TrendBadge 用 lucide 箭头显示与上一周期对比；TanStack Query 拉数据并监听 `stats-updated` 事件做 `invalidateQueries` 实现零延迟刷新。
- 前置：P3-5, P0-8
- 验收：四周期切换正确，事件触发时数据自动更新，趋势方向正确。
- PRD Story：16-25

### P3-7 历史与月度归档前端
- 所属层：React
- 文件：`src/pages/Statistics/index.tsx`（历史子视图）或 `src/pages/Statistics/_components/HistoryPanel.tsx`
- 实现要点：列出可归档月份（Select/列表），展示月度数据，提供删除按钮（Dialog 二次确认 + sonner 反馈）。
- 前置：P3-5
- 验收：可查看与删除月度归档，删除后列表刷新。
- PRD Story：78, 79, 80

## 参考材料（origin）

> 路径相对 `docs/origin/ccNexus/`

| 关注点 | 文件 | 说明 |
|--------|------|------|
| 统计记录入口 | `internal/proxy/stats.go` | 请求/错误/token 记录时机 |
| 四周期 / 趋势 / 防抖保存 | `internal/service/stats.go` | `scheduleSave` 2s、周期聚合、趋势对比 |
| daily_stats 读写 | `internal/storage/stats_adapter.go` | upsert 累加、区间聚合 |
| 凭证用量 | `internal/storage/credential_usage.go` | `credential_usage` 表 |
| 月度归档 | `internal/service/archive.go` | 归档查询 / 删除 |
| usage 提取测试样本 | `internal/proxy/{token_extraction_test,streaming_usage_test}.go` | input/output token 提取用例 |
| 前端统计交互 | `cmd/desktop/frontend/src/modules/stats.js` | 周期切换 / 趋势展示 UX |

## 完成判据（里程碑 M3 之一）

- upsert 累加、周期边界、趋势、防抖单测通过（详见 P10-2）；
- 连续记录 2s 内只落库一次，前端收到 `stats-updated` 并零延迟刷新；
- 四周期 + 历史归档前端可用。
