# cc-switch 真实设计调研（参考项目：E:\myCode\cc-switch）

> 结论先行：需求摘要 `协议转换功能需求.txt` 对 cc-switch 的描述与源码**有出入**。本文以源码为准，
> 所有 file:line 指向 `E:\myCode\cc-switch\src-tauri\src\proxy\`。

## 0. 需求摘要 vs 源码真相

| 需求摘要的说法 | cc-switch 源码真相 | 证据 |
|---|---|---|
| Rectifier 拦截 `Invalid pages parameter:""`、剥离空字段重试 | **不存在**。全仓无工具参数空字段剥离 | grep 无 strip_empty/remove_empty 相关工具逻辑 |
| "JSON canonicalization 过滤空值" | 实为**排序键**(缓存稳定) + 空 `arguments` 整串 `""`→`"{}"` | json_canonical.rs:70-75 |
| Rectifier = 错误拦截 + 清洗参数 + 重试 | Rectifier 确为错误驱动重试，但修的是 **thinking 签名 / thinking budget / media**，靠匹配错误消息 | thinking_rectifier.rs:26-109 |
| 会话级 Map<call_id,toolu_id> 跨轮恢复 | 实为 `CodexChatHistoryStore`，**Codex Responses API → Chat 桥接专属** | codex_chat_history.rs:31-45 |

## 1. 工具参数 canonicalization（→ 本项目 A）

源文件 `json_canonical.rs`（190 行，可整体移植）。核心函数：

- `canonicalize_value(Value)->Value`（:6）：递归把 object 的键**排序**（稳定序列化）。
- `canonical_json_string(&Value)->String`（:23）：产出排序键的紧凑 JSON 串；object→`{...}`、空 object→`"{}"`、null→`"null"`。
- `canonicalize_json_string_if_parseable(&str)->String`（:51）：串可解析为 JSON 则规范化，否则原样返回（保护纯文本）。
- `canonicalize_tool_arguments_str(&str)->String`（:70）：同上，但**空/纯空白串强制 `"{}"`**。docstring 点名 bug：严格上游(Minimax)拒绝 `arguments:""` 报 400 `invalid function arguments json string`；宽松上游(OpenAI/Kimi)静默当空对象。
- `canonicalize_tool_arguments(Option<&Value>)->String`（:82）：String→上面；结构化值→canonical_json_string；缺失→`"{}"`。

**应用点（证明是主动/请求构建期，不是反应式）：**
- `providers/transform.rs:411` —— Anthropic `tool_use` → OpenAI `tool_calls`：`"arguments": canonical_json_string(&input)`（input 缺省 `json!({})`）。
- `providers/transform.rs:424` —— `tool_result` 结构化 content → `canonical_json_string(v)`。
- `providers/streaming_codex_chat.rs:723` —— 流式累积参数收尾：`canonicalize_tool_arguments_str(&state.arguments)`。
- forwarder.rs:1170 注释明确："避免上游拒绝后由 rectifier 反应式重试（首次请求已消耗 quota）"→ **cc-switch 哲学是优先主动 sanitize，少走反应式**。

## 2. thinking 签名 Rectifier（→ 本项目 B）

源文件 `thinking_rectifier.rs`（722 行；核心 1-242，其余为测试）。

- `should_rectify_thinking_signature(error_message: Option<&str>, &RectifierConfig)->bool`（:26-109）：
  先查 `config.enabled && config.request_thinking_signature`；再对**错误消息小写 substring 匹配** 7 类场景：
  1. invalid + signature + thinking + block
  2. "thought signature" + (not valid|invalid)
  3. "must start with a thinking block"
  4. expected + (thinking|redacted_thinking) + found + tool_use
  5. signature + "field required"
  6. signature + "extra inputs are not permitted"
  7. thinking|redacted_thinking + "cannot be modified"
  8. 非法请求 / illegal request / invalid request（兜底）
  测试证明：错误体是**嵌套 JSON 串也能命中**（纯 substring，无需解析结构）—— 见 :308-324。所以传**原始错误体文本**即可。

- `rectify_anthropic_request(&mut Value)->RectifyResult`（:118-189）：**原地改 Anthropic 请求体**。
  - 遍历 `messages[].content[]`：删 `type==thinking` / `redacted_thinking` 块；删任意块上的 `signature` 字段。
  - 末尾 `should_remove_top_level_thinking`（:192-237）：`thinking.type=="enabled"` 且最后一条 assistant 消息首块不是 thinking/redacted_thinking 且含 tool_use → 删顶层 `thinking` 字段。
  - 返回 `RectifyResult{applied, removed_thinking_blocks, removed_redacted_thinking_blocks, removed_signature_fields}`。
- `normalize_thinking_type(Value)->Value`（:240）：当前**空实现**（请求前不主动改 thinking type）。

**RectifierConfig**（`proxy/types.rs:202-247`）字段：`enabled` / `request_thinking_signature` / `request_thinking_budget` / `request_media_fallback` / `request_media_heuristic`，**Default 全 true**。

**forwarder.rs 重试编排**（:392-950，要点，不逐行抄）：
- 每种 rectifier 一个**一次性**标志：`rectifier_retried` / `budget_rectifier_retried` / `media_rectifier_retried`。
- 上游错误 → 取错误消息 → `should_rectify_thinking_signature(msg, cfg)` 为真且未重试过 → `rectify_anthropic_request(&mut provider_body)` → 若 `applied` 置标志并**重试同请求**；否则标记 `signature_rectifier_non_retryable_client_error`。
- `handle_rectifier_retry_failure`（:256）：统一处理 rectifier 重试仍失败的日志/收尾。

**关键适配认知（本项目）**：`rectify_anthropic_request` 改的是 **Anthropic 格式 body**（找 messages[].content[] thinking 块）。
→ 它对 **Claude 入站→Claude 上游 passthrough** 路径最有用（thinking+signature 被原样转发、被第三方 Claude 兼容后端拒绝）。
对 OpenAI-transform 路径意义不大（thinking 已在转换中被处理掉、signature 不外发）。

## 3. 为何放弃 D（会话 ID 缓存）

`providers/codex_chat_history.rs:31-45` docstring 原文：
> "Cross-request history needed when **Codex Responses is bridged to Chat Completions**. ... Codex often sends follow-up requests as `previous_response_id + function_call_output`, so this store restores the missing function call before the request is converted to Chat messages."

即：只因 **Codex Responses API**（`/v1/responses`）用 `previous_response_id` 做服务端状态、不发完整历史，才需要重建丢失的 function_call。
本项目 tauri-gateway 只做 **Claude Messages ↔ OpenAI Chat Completions**，客户端每轮发完整历史、ID 原样透传、自洽。
`session.rs:1-10` 的 Session ID 提取(metadata.user_id)也是 cc-switch 通用基建，本项目 Chat 路径无需 ID 重映射。
→ 建 ID 缓存属过度设计，**不做**。

## 4. 流式多工具 by index（→ 本项目 C，借鉴思路非照搬）

cc-switch `providers/streaming.rs`（1143 行，异步 stream 架构，与本项目 `StreamConverter` 同步 chunk 驱动**不同构**）。可借鉴的数据结构：
- `DeltaToolCall { index: usize, id: Option<String>, type, function }`（:43-48）—— OpenAI 流每个 tool delta 带 `index`。
- `ToolBlockState { anthropic_index: u32, id, name, started, ... }`（:85-90）。
- `tool_blocks_by_index: HashMap<usize, ToolBlockState>`（:161）—— **OpenAI index → Anthropic 块状态**映射。
- `open_tool_block_indices: HashSet<u32>`（:162）、`next_content_index: u32`（:147）。

**借鉴要点**：用 OpenAI `tool_calls[].index` 作键区分多工具，首次见某 index 才分配新的 Anthropic content block 并发 `content_block_start`，后续 `arguments` 片段按 index 路由到对应块的 `input_json_delta`。
**不照搬**其异步架构——本项目改 `StreamConverter`/`StreamContext` 即可（详见 feature.md C 任务）。
