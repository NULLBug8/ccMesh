# 本项目现状与集成落点（tauri-gateway）

> 基于两个 Explore 子 agent 报告 + 主会话亲读核对（行号已按实际源码校准）。
> 路径前缀 `src-tauri/src/modules/`。

## 1. transform 层现状（transform/）

全部基于 `serde_json::Value`，**无工具调用的类型化 struct**；仅有 `StreamContext`/`UpstreamFormat`/`Transformer`/`StreamConverter`。

| 能力 | 状态 | 位置 |
|---|---|---|
| 请求 tool_use→tool_calls（arguments=`to_string(input)`） | 有，但**未规范化** | claude_openai.rs:117-130（:124 是落点） |
| tool_choice 映射（tool/any/auto） | 有 | claude_openai.rs:164-177 |
| tools input_schema→function.parameters | 有 | claude_openai.rs:71-94 |
| tool_result→role:"tool" | 有 | claude_openai.rs:131-140 |
| 响应 tool_calls→tool_use（arguments 串 parse） | 有，parse 失败静默吞为 `{}` | claude_openai.rs:199-220（:214） |
| finish_reason→stop_reason | 有 | types.rs:51-61 |
| 流式 tool delta | 有但**单工具**，按 id 出现判定、忽略 `index` | streaming.rs:139-179 |
| 参数 canonicalization / 空 arguments→{} | **无** | —— |
| thinking 签名 rectify | **无** | —— |

**StreamContext**（types.rs:5-21）当前单工具字段：`current_tool_id/current_tool_name/tool_arguments`（String）+ `block_index`。
**入口**：`get_transformer(format).transform_request(cj, Some(&ep.model))`（transformer.rs:45）；`openai_response_to_claude`（claude_openai.rs:180）；`StreamConverter::{new,process_chunk,finish,usage}`（streaming.rs）。

## 2. proxy 转发现状（proxy/forward.rs）

主处理器 `handle_proxy`（:174-460），单函数内含轮换/熔断/重试 `for _ in 0..max` 循环（:284）。reqwest 客户端，axum 服务。

- 请求体解析：`body_json: Option<Value>`（:185，原始 Claude 体）、`model`、`client_wants_stream`。
- 每轮顶部按 `needs_transform` 重建 `attempt_body`（:309-336）：转换走 `transform_request`，否则 model-lock 或原样。
- 发上游 `send_upstream`（:345→462）。
- status==200 → 4 路响应处理（:378-383）：`stream_transform_response` / `transform_buffered_response` / `relay_stream_response` / `relay_buffered_response`。
- **非 200**（:384-416）：按 `categorize_status` 记熔断；`should_retry_status`（rotation.rs:55-57，除 200/400/401 都重试）为假（即 400/401）→ **`relay_passthrough(resp)` 原样回传，错误体从不读取/解析**（:401-408 + :563-571）。
- 重试触发：状态码、连续失败切换(`CONSECUTIVE_FAIL_SWITCH=2`)、瞬时网络错误延迟重试。
- **无任何会话级/对话级状态**；`ProxyState`（:68-85）只有端点级状态（rotation/active/breakers/current_endpoint）。

## 3. 三项集成的精确落点

### A 主动 canonicalization
- 新建 `transform/json_canonical.rs`（移植 cc-switch json_canonical.rs；改 `pub(crate)`→模块内 `pub`，去掉 sha2 相关如不需要）。
- 改 `claude_openai.rs:124`：`serde_json::to_string(&input)` → `canonicalize_tool_arguments(Some(&input))`（空→`{}`、排序键）。
- 改 `claude_openai.rs:131-140` tool_result 结构化 content → `canonical_json_string`。
- 改 `streaming.rs` 工具参数收尾：累积串经 `canonicalize_tool_arguments_str` 规整（注意流式是增量 `partial_json`，规范化只能在**整块收尾**做，不能逐片排序——见 feature.md 说明）。
- `transform/mod.rs` 加 `pub mod json_canonical;`。

### B thinking 签名 rectifier
- 新建 `transform/thinking_rectifier.rs`（移植 should_rectify_thinking_signature + rectify_anthropic_request + should_remove_top_level_thinking + RectifyResult）。
- 新建轻量 `RectifierConfig{enabled, request_thinking_signature}`（本项目只做签名，砍掉 budget/media 字段），可放 thinking_rectifier.rs 或 models/config.rs。
- 改 `forward.rs` 重试循环：在 `should_retry_status==false`（:401）**之前/之中**，对 4xx 先**缓冲读取错误体文本**；若 `should_rectify_thinking_signature(&body_text, &cfg)` 且本轮未 rectify 过 → 对 `body_json`（需改 `mut` 或用 override 变量）调 `rectify_anthropic_request`，`applied` 则 `continue` 重试（下一轮顶部用改后的 body_json 重建 attempt_body）；否则用**缓冲的字节**重建 Response 回传（因 resp 已被 consume，不能再 relay_passthrough）。
- 一次性标志 `sig_rectified: bool` 防死循环。
- 配置来源：`ProxyState` 加字段或读全局 config（start_proxy 时注入），默认 enabled。

### C 流式多工具 index
- 改 `StreamContext`（types.rs）：用 `tool_blocks: HashMap<i64, ToolBlock>`（key=OpenAI index）替代/补充单 `current_tool_*`；`ToolBlock{ anthropic_index:i64, id, name, started }`。
- 改 `streaming.rs:handle_tool_call`：读 `tc["index"]`（缺省 0）；首见该 index→分配 anthropic block、发 content_block_start、入表；后续 args 片段按 index 找到对应块发 input_json_delta。需协调 text/thinking 块与多 tool 块的 index 分配与关闭顺序。
- 收尾 `finish`/`close_open_blocks`：关闭所有仍开着的 tool 块。

## 4. 现有测试基线（移植/新增测试参考）
- claude_openai.rs:315-451 已有请求/响应/tool_choice 测试。
- streaming.rs:315-383 已有文本流/工具流/usage 测试。
- 验证命令：`cargo test`（库）、`cargo check`、`cargo build`（详见 feature.md Run 段）。
