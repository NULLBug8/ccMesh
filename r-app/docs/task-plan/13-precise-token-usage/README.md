# 阶段 13：精确 Token 计数（响应真实 usage）+ P4 占位补全

> 来源：[`../task.md`](../task.md)。参考实现：`E:\myCode\cc-switch`（`src-tauri/src/proxy/usage/parser.rs` 的 `TokenUsage`）。
> 进度跟踪见同目录 [`progress.csv`](./progress.csv)。定稿日期：2026-06-07。

## 一、背景与问题

调研发现两处缺陷：

1. **统计 token 永远为 0**：`forward.rs` 转发成功时 `st.stats.record(&ep.name, false, 0, 0)` 把 input/output token 硬编码为 0——统计面板的 token 数据全程为 0，毫无意义。
2. **P4-8 HTTP 端点占位**：`server.rs` 的 `count_tokens_route`（`/v1/messages/count_tokens`）返回写死的 `{"input_tokens":0}`，未接入已实现的 `estimate_input_tokens`。

cc-switch 的 token 精度来自**解析上游响应的真实 usage 字段**（而非本地估算）：`TokenUsage::from_claude_response`（`usage.input_tokens/output_tokens`）、`from_openai_response`（`usage.prompt_tokens/completion_tokens`），并支持流式（Claude `message_start`+`message_delta`、OpenAI 末 chunk usage）。本阶段采用同一思路适配本项目。

## 二、设计

### 2.1 不引入 tokenizer 库
精度来自**上游返回的真实计费 token**，与 cc-switch 一致；不引入 tiktoken（Claude 非 tiktoken、且会增加体积与词表依赖）。`count_tokens`（请求前预估）本质是估算，保持现有字符近似，仅补全 HTTP 端点占位。

### 2.2 新模块 `modules/usage.rs`
统一按端点上游格式（`UpstreamFormat`）解析 usage：
- `from_response(body, format) -> (input, output)`：非流式。Claude 读 `usage.input_tokens/output_tokens`；OpenAI 读 `usage.prompt_tokens/completion_tokens`。
- `UsageAccumulator{ format, buf, input, output }`：流式 SSE 累积器。`feed(&[u8])` 按行解析 `data:`；`finish() -> (input, output)`。
  - Claude：`message_start.message.usage.input_tokens`、`message_delta.usage.output_tokens`。
  - OpenAI：末 chunk `usage.prompt_tokens/completion_tokens`。

### 2.3 转发响应接入（`forward.rs`）
解析格式 = 端点 `UpstreamFormat`（上游响应即该格式）。200 分支不再硬编码 record 0，按 4 条路径在响应处理后 record 真实 token：

| needs_transform | client_wants_stream | 处理 | usage 来源 |
|---|---|---|---|
| false | false | 缓冲直通 | `from_response`（按 format） |
| false | true | SSE tap 转发 | `UsageAccumulator`（按 format） |
| true | false | OpenAI→Claude 缓冲 | `from_response`（OpenAI） |
| true | true | OpenAI SSE→Claude | `StreamConverter` 已累积，新增 getter |

错误（非 200）响应走纯直通 `relay_passthrough`，record 在外保持（error, 0, 0）。

### 2.4 `StreamConverter` 复用
`process_chunk` 已把 OpenAI `prompt_tokens/completion_tokens` 累积到 `ctx`，新增 `usage()->(i64,i64)` getter，流结束读出 record。

### 2.5 P4-8 端点补全
`server.rs::count_tokens_route` 改为读 body 的 `system`/`messages`，调 `tokens::estimate_input_tokens` 返回真实估算。

## 三、任务（见 progress.csv）

- **P13-1** usage 解析模块 + StreamConverter getter
- **P13-2** 转发接入真实 usage（4 路径 record，错误 passthrough）
- **P13-3** count_tokens HTTP 端点补占位
- **P13-4** 测试与回归

## 四、验收

- 非流式/流式、Claude/OpenAI 端点转发后，统计写入真实 input/output token（非 0）
- `/v1/messages/count_tokens` 返回与 IPC `count_tokens` 一致的估算
- 既有转发/转换行为不回归，测试通过
