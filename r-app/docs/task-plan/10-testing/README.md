# 阶段 10：测试

> 任务总线：[../../TASKS.md](../../TASKS.md) ｜ 规划索引：[../README.md](../README.md)
> 起点参考：旧版 ccNexus 源码 `docs/origin/ccNexus/`（Go + Wails，仅参考不复用）

## 阶段目标

覆盖核心逻辑的单元测试（转换器/统计/存储/轮换/解析）、集成测试（代理端到端、WebDAV），以及前端交互测试（mock IPC）。交付 **里程碑 M6 发布就绪** 的质量保障。旧版 `*_test.go` 是用例样本的主要来源。

## 前置依赖

- 各被测阶段：1（代理）、2（转换）、3（统计/存储）、5（WebDAV）、6/7/9（前端）。

## 任务清单

### P10-1 转换器单元测试
- 所属层：Rust
- 文件：`src-tauri/src/modules/transform/`（`#[cfg(test)]`）
- 实现要点：覆盖 Claude ↔ OpenAI 非流式、tool_use、reasoning、流式 chunk、usage 提取；用旧版 `*_test.go` 用例作参考样本。
- 前置：P2-2~P2-5
- 验收：`cargo test` 转换相关用例全绿。
- PRD Story：Testing（转换器）、11-15

### P10-2 统计与存储单元测试
- 所属层：Rust
- 文件：`src-tauri/src/modules/stats/`、`src-tauri/src/modules/storage/`（`#[cfg(test)]`，临时 DB）
- 实现要点：周期边界、趋势对比、防抖只落库一次、upsert 累加、迁移幂等、归档删除。
- 前置：P3-1~P3-4, P0-4
- 验收：`cargo test` 统计/存储用例全绿。
- PRD Story：Testing（统计/存储）、16-30, 78-80

### P10-3 轮换与解析单元测试
- 所属层：Rust
- 文件：`src-tauri/src/modules/proxy/`（`#[cfg(test)]`）
- 实现要点：轮换循环、连续失败 2 次切换、瞬时错误重试延迟、最大重试次数、三种端点解析。
- 前置：P1-2, P1-3
- 验收：`cargo test` 代理逻辑用例全绿。
- PRD Story：Testing（代理）、2-8

### P10-4 代理集成测试
- 所属层：Rust
- 文件：`src-tauri/tests/proxy_integration.rs`
- 实现要点：用 mock 上游（wiremock 或本地 axum stub）验证端到端转发、故障转移、格式转换、usage 统计写入。
- 前置：P2-6, P3-3
- 验收：集成测试通过，覆盖正常与故障路径。
- PRD Story：Testing（集成）、1-15, 24

### P10-5 WebDAV 集成测试
- 所属层：Rust
- 文件：`src-tauri/tests/webdav_integration.rs`
- 实现要点：对接本地/测试 WebDAV，验证连接、备份、恢复、列表、删除、设备过滤。**注：正式 WebDAV 测试环境暂未提供，先以本地 stub（如 dufs / rclone serve webdav）占位运行，后续接入正式测试端点再补充。**
- 前置：P5-3, P5-4
- 验收：集成测试通过。
- PRD Story：Testing（WebDAV）、31-39

### P10-6 前端集成/交互测试
- 所属层：React
- 文件：`src/__tests__/`（Vitest + Testing Library，按需添加为 devDependencies）
- 实现要点：mock `@tauri-apps/api` 的 `invoke/listen`（或 mock `src/services` 层），测试端点筛选/克隆/测试交互、统计事件刷新、更新红点逻辑、主题与语言切换；被测组件主要位于 `pages/*/_components/` 与 `components/business/`（如 `UpdateBadge`）。
- 前置：P7-x, P3-6, P9-3, P6-x
- 验收：关键交互用例通过。
- PRD Story：Testing（前端交互）

## 参考材料（origin）

> 路径相对 `docs/origin/ccNexus/`

| 关注点 | 文件 | 说明 |
|--------|------|------|
| 转换器用例样本 | `internal/transformer/convert/{claude_openai_test,claude_openai2_test,openai_openai2_test}.go` | 直接迁为 Rust 单测样本 |
| 请求/响应用例 | `internal/proxy/request_test.go` | 请求构造断言 |
| 流式 usage 用例 | `internal/proxy/streaming_usage_test.go` | SSE usage 提取 |
| token 提取用例 | `internal/proxy/token_extraction_test.go` | token 计数断言 |

## 完成判据（里程碑 M6 之一）

- `cargo test` 单元 + 集成全绿（转换/统计/存储/轮换/代理/WebDAV）；
- 前端关键交互（筛选/克隆/测试/统计刷新/更新红点/主题语言）用例通过。
