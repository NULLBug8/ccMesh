# 07 — WP7 监控端点路由展示

> 关联：[TASKS.md](./TASKS.md) · [PRD-2.md](./PRD-2.md)
> 所属层：后端（Rust/SQLite）+ 前端
> 前置：WP5（前端类型基线）

## 目标

让实时请求监控的"入站/出站"列展示**真实端点路由路径**（R1）：入站如 Claude `/v1/messages`、OpenAI `/v1/chat/completions`；出站为实际转发上游路径（转换后为 `/v1/chat/completions`）。当前仅记录 `inbound_format`（协议词）与 `upstream_url`（base URL），故需在明细表新增真实路径两列并由采集点透传。

## 现状（根因）

- `request_logs` 仅有 `inbound_format`、`upstream_url`；`forward.rs::handle_proxy` 持有真实 `uri.path()` 与 `upstream_path` 但未落库。
- 入站协议在 `RequestMeta.inbound_format` 取 `openai`/`claude`；出站 `upstream_url = ep.api_url`（base）。

## 关键文件/落点

- 迁移：`src-tauri/src/modules/storage/migration.rs`（追加 **v5**：`ALTER TABLE request_logs ADD COLUMN inbound_path TEXT` / `upstream_path TEXT`）
- 模型：`src-tauri/src/models/stats.rs`（`RequestLog` 增两列）
- 仓储：`src-tauri/src/modules/storage/request_logs_repo.rs`（`insert_batch` 列、`row_to_log`、`query_page` SELECT 列）
- 聚合/采集：`src-tauri/src/modules/stats/aggregator.rs`（`RequestRecord`、`record` 构造 `RequestLog`）、`src-tauri/src/modules/proxy/forward.rs`（`RequestMeta` 透传 `inbound_path`=`uri.path()`、`upstream_path`=`upstream_path`）
- 前端类型：`src/services/modules/stats.ts`（`RequestLog` 增 `inboundPath`/`upstreamPath`）
- 前端展示：`src/components/business/RequestMonitor.tsx`（`RequestLogTable`/`RequestRow` 入站、出站列）
- 测试：`src-tauri/.../request_logs_repo.rs`（tests）、`migration.rs`（tests）、`src/__tests__/RequestMonitor.test.tsx`

## 任务拆解

- **7.1** v5 迁移 + 模型 + 仓储：
  - migration v5：为 `request_logs` 增 `inbound_path TEXT`、`upstream_path TEXT`（可空）。
  - `RequestLog` 模型增两字段；`insert_batch` 增列与占位符；`row_to_log` 增列读取；`query_page` SELECT 增列。注意旧行为 NULL → 反序列化为 `Option<String>`/空串（实现时统一）。
- **7.2** 采集点透传：
  - `RequestMeta` 增 `inbound_path: String`、`upstream_path: String`；`handle_proxy` 构造 `RequestMeta` 时填 `uri.path()` 与该次 attempt 的 `upstream_path`。
  - `RequestRecord` 增两字段；`StatsAggregator::record` 写入 `RequestLog` 时带上；`request-logged` 事件 payload 自然包含（结构体增字段即可）。
  - 失败兜底分支（无 upstream 的 `record`）`upstream_path` 给空。
- **7.3** 前端展示：
  - `stats.ts` 的 `RequestLog` 增 `inboundPath: string | null`、`upstreamPath: string | null`。
  - `RequestRow` 入站列：优先展示 `inboundPath`，为空时按 `inboundFormat` 兜底（claude→`/v1/messages`、openai→`/v1/chat/completions`）；协议词降级为辅助（title/副标题）。
  - 出站列：优先展示 `upstreamPath`，为空时兜底；完整上游 URL（`upstreamUrl`）保留为 `title` 悬停。

## 数据契约（增字段，向后兼容）

```
RequestLog {
  …(原有字段),
  inbound_path: Option<String>,   // 真实入站路径，如 /v1/messages
  upstream_path: Option<String>,  // 真实出站路径，如 /v1/chat/completions
}
```
- `get_request_logs` / `request-logged` 仅"增字段"，旧消费者不受影响。

## 验收标准

- 新发生的请求：入站列显示真实入站路径，出站列显示真实出站路径（含 Claude→OpenAI 转换后为 `/v1/chat/completions`）。
- 迁移前旧行：路径列按协议推断兜底展示，不空白、不报错。
- 完整上游 URL 仍可通过悬停获取。
- v5 迁移可从 v4 旧库平滑升级。

## 测试点

- 后端 `request_logs_repo`（扩展）：插入含 `inbound_path/upstream_path` 的行并回读一致；含空值（旧行模拟）正常；分页查询返回新列。
- 后端 `migration`（扩展）：v5 后 `request_logs` 含两新列；v4→v5 平滑升级（仿 `v3_adds_cache_columns_and_request_logs`）。
- 前端 `RequestMonitor.test.tsx`（扩展）：给定含路径的 log 渲染路径列；缺省时按 `inboundFormat` 渲染兜底路径。
