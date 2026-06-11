# moon-bridge 调研：OpenAI Responses 路由 / 协议转换

> 参考项目：`E:\myCode\reference data\moon-bridge`（Go 语言）
> 用途：作为 tauri-gateway 实现 `/v1/responses` 路由 + Responses→Chat 转换 + Responses 透传的**正确性校对基准**。
> 调研范围：路由注册、Responses→Core→Chat 字段映射、流式事件转换、透传分支判断、codex 专属处理。

---

## 0. 总体架构（关键认知）

moon-bridge **不是** 直接 Responses→Chat 单跳转换，而是经过一个**中间格式 Core（`internal/format`）**做双向桥接：

```
入站 OpenAI ResponsesRequest
  → ClientAdapter.ToCoreRequest()        (openai 包)  → format.CoreRequest
  → ProviderAdapter.FromCoreRequest()    (chat 包)    → chat.ChatRequest   （= Chat Completions 请求）
  → 上游 chatClient.CreateChat/StreamChat → chat.ChatResponse / chunk 流
  → ProviderAdapter.ToCoreResponse()     (chat 包)    → format.CoreResponse
  → ClientAdapter.FromCoreResponse()     (openai 包)  → openai.Response    （= Responses 响应）
```

- 入站永远是 `openai-response` 协议；出站协议由路由解析的候选 provider 的 `Protocol` 决定（`anthropic` / `openai-chat` / `google-genai` / `openai-response`）。
- Responses→Chat 的字段映射因此**拆成两段**：Responses↔Core 在 `internal/protocol/openai/adapter.go`，Core↔Chat 在 `internal/protocol/chat/adapter.go`。下方映射表已合并这两段，直接给出 Responses↔Chat 的等效结果。

关键文件：
- 路由注册：`internal/service/server/server.go:152-155`
- 总分发 + **透传判断**：`internal/service/server/dispatch.go`（`handleResponses` / `handleOpenAIResponse`）
- 适配器分发（Responses→Chat/Anthropic/Google）：`internal/service/server/adapter_dispatch.go`（`handleWithAdapters` / `handleAdapterStream`）
- Responses↔Core：`internal/protocol/openai/adapter.go` + `types.go`
- Core↔Chat：`internal/protocol/chat/adapter.go` + `types.go`
- 协议常量：`internal/config/config.go:20-24`
- codex 工具语法：`internal/extension/codextool/customtool.go`
- codex 模型目录：`internal/extension/codex/catalog.go`

---

## 1. responses 路由 / 转发

### 1.1 路由注册（`server.go:152-155`）

```go
s.mux.HandleFunc("/v1/responses", s.handleResponses)
s.mux.HandleFunc("/responses", s.handleResponses)    // 同时注册带/不带 /v1 前缀
s.mux.HandleFunc("/v1/models", s.handleModels)        // codex 客户端拉模型目录用
s.mux.HandleFunc("/models", s.handleModels)
```

**没有独立的 "codex" 路由或 "codex" 协议**——codex CLI 走的就是标准 `/v1/responses`（Codex `wire_api = "responses"`，见 `catalog.go:537`）。

### 1.2 handler 主流程（`dispatch.go:handleResponses`）

1. 仅接受 `POST`，否则 405。
2. 读 body → `json.Unmarshal` 到 `openai.ResponsesRequest`（`dispatch.go:81`）。
3. `resolveModelOrFallback(req.Model)` 解析模型路由 → 得到候选 provider 列表。
4. `filterCandidatesByInput`：按输入特征（如是否含图片）过滤候选。
5. 取 `preferred` 候选，**按其 `Protocol` 分支**：
   - `Protocol == openai-response` → `handleOpenAIResponse`（**透传**，见第 3 节）。
   - 其它协议（anthropic/openai-chat/google-genai）→ `handleWithAdapters`（**转换**）。

协议常量（`config.go:20-24`）：
```go
ProtocolAnthropic      = "anthropic"
ProtocolOpenAIResponse = "openai-response"
ProtocolGoogleGenAI    = "google-genai"
ProtocolOpenAIChat     = "openai-chat"
```

---

## 2. Responses API → Chat Completions 转换（重点）

### 2.1 请求体字段映射表（Responses → Chat）

合并 `openai.ToCoreRequest`（adapter.go:70-175）与 `chat.FromCoreRequest`（chat/adapter.go:63-148）两段后的等效映射：

| Responses 字段 | Core 中转 | Chat Completions 字段 | 转换说明 / 注意点 |
|---|---|---|---|
| `model` | `CoreRequest.Model` | `model` | 分发层会用上游模型名覆盖（`coreReq.Model = preferred.UpstreamModel`，adapter_dispatch.go:171）。 |
| `input` (string) | 单条 `user` message | `messages:[{role:user, content:"..."}]` | 字符串输入 → 单个 user 文本消息（adapter.go:962-976）。 |
| `input` (array) | `Messages` + `System` | `messages[]` | 按 item 类型展开；详见 2.3 input 数组规则。 |
| `instructions` | 插到 `System` **最前** | 合并进 `messages[0]` 的 `system` 文本 | `instructions` 优先级最高，prepend 到 system 块首（adapter.go:92-97）。Chat 端把所有 system 块拼成**单条** `role:"system"` 消息（chat/adapter.go:85-93）。 |
| `max_output_tokens` | `CoreRequest.MaxTokens` | **`max_completion_tokens`** | 注意 Chat DTO 用的是 `max_completion_tokens`（chat/types.go:17），不是旧的 `max_tokens`。Core MaxTokens 为 0 时回退到 `cfgMaxTokens`（chat/adapter.go:108-112）。 |
| `temperature` | `CoreRequest.Temperature` (*float64) | `temperature` | 指针直传，nil 即不设。 |
| `top_p` | `CoreRequest.TopP` (*float64) | `top_p` | 同上。 |
| `tools[]` | `CoreRequest.Tools` | `tools[]` (type:function) | function 工具：`name/description/parameters→InputSchema`。web_search/file_search/custom 等特殊类型在 openai 端先展开（见 2.4 + 第 4 节 codex）。Chat 端统一转成 `{type:"function", function:{name,description,parameters}}`（chat/adapter.go:118-130）。 |
| `tool_choice` | `CoreToolChoice{Mode,Name,Raw}` | `tool_choice` | openai 端解析 string("auto"/"none"/"required") 或 object，**保留原始 Raw**（adapter.go:1269-1308）。Chat 端**优先回写 Raw**，否则按 Mode 重建（chat/adapter.go:133-139, toChatToolChoice）。 |
| `parallel_tool_calls` | `Extensions["openai"]["parallel_tool_calls"]` | `parallel_tool_calls` | 经扩展包透传（adapter.go:147-149）。注意 Chat DTO 有该字段但 FromCoreRequest 当前**未回写**它——潜在遗漏点。 |
| `reasoning.effort` | `Extensions["openai"]["reasoning"]` | **`reasoning_effort`** | Chat 端从扩展里取 `reasoning.effort` → `reasoning_effort`（chat/adapter.go:143-145, 152-163；字段 chat/types.go:26-29）。 |
| `stop` | （未映射） | `stop` | **注意**：openai.ResponsesRequest 有 `Stop` 字段（types.go:25），但 `ToCoreRequest` **没有把它写进 Core**；Core 的 `StopSequences` 来源另有路径。校对时留意。 |
| `prompt_cache_key` / `prompt_cache_retention` | `Extensions["cache"]` | （Chat 不直接用） | 缓存元数据，主要给 Anthropic 路径用。 |
| `metadata` | `CoreRequest.Metadata` | `metadata` | 直传。 |
| `text` / `service_tier` / `previous_response_id` / `store` / `include` | `Extensions["openai"][...]` | 多数不进 Chat | 仅做扩展透传，Chat 路径基本忽略。 |
| `stream` | `CoreRequest.Stream` | 决定走流式分支 | 控制 `handleWithAdapters` 是否进 `handleAdapterStream`。 |

### 2.2 响应体字段映射表（Chat → Responses）

合并 `chat.ToCoreResponse`（chat/adapter.go:173-217）与 `openai.FromCoreResponse`（adapter.go:186-307）：

| Chat 响应字段 | Core 中转 | Responses 字段 | 说明 |
|---|---|---|---|
| `id` | `CoreResponse.ID` | `response.id` | |
| — | — | `response.object = "response"` | 固定。 |
| `choices[0].finish_reason` | `StopReason` + `Status` | `response.status` | `length`→status `incomplete`；`content_filter`→`failed`；否则 `completed`。finish_reason 映射：stop→end_turn, length→max_tokens, tool_calls→tool_use（chat/adapter.go:872-885）。 |
| `choices[].message.content` | assistant text block | `output[].type="message"`, `content[].type="text"` + 顶层 `output_text` | 多个 text 拼接成 `output_text`（adapter.go:256-267）。 |
| `choices[].message.reasoning_content` | `reasoning` block | `output[].type="reasoning"`, `summary[].text` | DeepSeek 等的思维链；prepend 到 content（chat/adapter.go:792-798）。 |
| `choices[].message.tool_calls[]` | `tool_use` block | `output[].type="function_call"`（或 custom_tool_call/local_shell_call） | 出站 item 类型由 codex_tool_map 反查决定（见第 4 节）。`arguments` 去引号处理（chat/adapter.go:887-901 unquoteArguments）。 |
| `usage.prompt_tokens` | `Usage.InputTokens` | `usage.input_tokens` | |
| `usage.completion_tokens` | `Usage.OutputTokens` | `usage.output_tokens` | |
| `usage.total_tokens` | `Usage.TotalTokens` | `usage.total_tokens` | |
| `usage.prompt_tokens_details.cached_tokens` | `Usage.CachedInputTokens` | `usage.input_tokens_details.cached_tokens` | |
| — | `Extensions["output_tokens_details"]` | `usage.output_tokens_details.reasoning_tokens` | 仅当扩展里带该数据时填充（adapter.go:279-291）。 |
| `error` | `CoreResponse.Error` | `response.error` + status=failed | |

### 2.3 input 数组 → messages 规则（adapter.go:944-1170，校对重点）

- `role:"system"` 或 `role:"developer"` 的 item → 归入 **System 块**（Chat 端拼成单条 system 消息）。
- `role:"assistant"` → assistant 消息；`role` 缺省视为 `user`。
- 内容部件类型：`input_text/text/output_text`→text block；`input_image/image/image_url`→image block（含 data: URI 解析 mediaType）。
- **工具调用输入** item（`function_call` / `custom_tool_call` / `local_shell_call`）→ assistant 消息内的 `tool_use` block；**连续的 function_call 批处理进同一条 assistant 消息**（pendingFCBlocks）。
- **工具结果输出** item（`function_call_output` / `custom_tool_call_output` / `local_shell_call_output`）→ 独立的 `role:"tool"` Core 消息（tool_result block，带 `call_id`）。Chat 端转成 `role:"tool"` + `tool_call_id` + content（chat/adapter.go:623-638）。
- **reasoning** item → 缓存为 pendingReasoning，merge 到紧随的 assistant/tool_use 之前（保证 reasoning→tool_use 相邻性，o1/o3/o4 推理模型还会在 function_call 前补空 reasoning 占位，adapter.go:1063-1076）。
- assistant 消息**同时含 text + tool_calls 时**：若 text 为空，Chat 端把 `content` 置 nil（OpenAI 要求有 tool_calls 时 content=null 而非 ""）（chat/adapter.go:596-604）。
- tool_call 的 `arguments`：Chat 出站时被 `json.Marshal(string(...))` 包成 JSON 字符串（chat/adapter.go:611）；Chat 入站解析时用 `unquoteArguments` 去掉外层引号还原成原始 JSON 对象。**这是 Chat 协议 arguments 始终是字符串编码的关键点。**

### 2.4 流式事件转换

两段流式：
1. **Chat SSE → Core 事件**（`chat.ToCoreStream`，chat/adapter.go:248-535）：delta 直映射，无需做快照差分。维护 per-choice 状态机：role=assistant 触发 `content_block.started`；`reasoning_content` 与 `content` 之间做 block 切换（reasoning block↔text block）；tool_calls 按 `index`/到达顺序分配 block slot，args delta 先 `json.Unmarshal` 解引号；`finish_reason` 触发 `content_block.done`；usage 来自最后一个 chunk（需 `stream_options.include_usage`）。
2. **Core 事件 → Responses SSE**（`openai.FromCoreStream`，adapter.go:319-922）：产出标准 Responses 流事件序列，**带全局递增 `sequence_number`**：
   - 生命周期：`response.created` / `response.in_progress` / `response.completed` / `response.incomplete` / `response.failed`（payload 内嵌完整 `response` 快照）。
   - 文本：`response.output_item.added` → `response.content_part.added` → `response.output_text.delta`(多次) → `response.output_text.done` → `response.content_part.done` → `response.output_item.done`。
   - 推理：`response.reasoning_summary_part.added` → `response.reasoning_summary_text.delta` → `response.reasoning_summary_part.done`。
   - 工具调用：`response.output_item.added`(item type=function_call/custom_tool_call/local_shell_call) → `response.function_call_arguments.delta` → `response.function_call_arguments.done` → `response.output_item.done`。
   - item id 规则：text=`msg_item_{idx}`，reasoning=`rs_item_{idx}`，tool=`fc_item_{idx}`（adapter.go:427-469）。
   - Anthropic 的 `ping` 事件无 Responses 对应，静默丢弃。

---

## 3. Responses 透传（passthrough）

判断分支在 `dispatch.go:handleResponses`：

```go
if preferred.Protocol == config.ProtocolOpenAIResponse {
    server.handleOpenAIResponse(...)   // 透传，不做协议转换
    return
}
// 否则走 handleWithAdapters（转换）
```

即：**当解析到的候选 provider 的 `Protocol` 是 `openai-response` 时走透传**。透传逻辑（`handleOpenAIResponse`，dispatch.go:276-554）：

- 仅筛选 `Protocol == openai-response` 的候选，按顺序做 failover。
- 上游 URL：`baseURL` 去尾斜杠，若未以 `/v1/responses` 或 `/responses` 结尾则补 `/v1/responses`（dispatch.go:364-367）。
- 请求体几乎原样转发，只改 `model` 为上游模型名；可选注入 web_search 工具（dispatch.go:369-376）。
- Header：`Content-Type: application/json` + `Authorization: Bearer <apiKey>`。
- 响应：直接 `io.Copy` 上游 body 到客户端（流式/非流式都透传原始字节），用 `io.MultiWriter` 旁路捕获用于 trace + usage 统计。**不解析、不重组 SSE**。

---

## 4. codex 特殊处理

**没有 "codex" 协议分支**；codex 特殊性集中在两处：工具语法代理（codextool）和模型目录生成（codex 包）。

### 4.1 codextool —— 工具语法双向重写（核心）

入站 `ToCoreRequest` 调 `flattenToolsWithNamespace`（adapter.go:1526-1626 convertToolWithNamespace），对 codex 特有工具做展开，并通过 `AnnotateCoreTool` 在 CoreTool.Extensions 写入 `codex_tool_kind / codex_openai_name / codex_namespace`，再把整张映射 `BuildToolMapFromCore(...).Encode()` 存进 `coreReq.Extensions["codex_tool_map"]`，供响应侧反查。

工具种类（`customtool.go`）：
- **apply_patch**（freeform grammar，`IsApplyPatchGrammar` 识别 `*** Begin Patch`/`*** End Patch`/`*** Add File:`）→ 展开为 5 个结构化代理工具 `apply_patch_{add_file,delete_file,update_file,replace_file,batch}`（带 JSON Schema），上游模型用结构化 JSON 调用；响应侧 `RebuildApplyPatchGrammar` 把结构化参数**重建回原始 apply_patch 文本语法**。可通过 `disablePatchProxy` 关闭代理。
- **exec**（`IsExecGrammar` 识别 `@exec` / pragma_source）→ 代理为带 `source` 字段的结构化工具；`RebuildExecGrammar` 还原。
- **local_shell**（custom，name=="local_shell"）→ 用 `LocalShellSchema`（command/working_directory/timeout_ms/env）；出站 item type=`local_shell_call`，带 `action`。
- **namespace** 工具 → 递归扁平化，名称拼接 `namespace_name`（`NamespacedToolName`）。
- 其它 custom → ToolRaw，保留原名 + raw input schema。
- 出站时 `OutputItemFromBlock`（customtool.go:101-122）按 tool_map 反查，决定输出 item 类型是 `function_call` / `custom_tool_call` / `local_shell_call`，并用 `RebuildGrammar` 还原 input。

### 4.2 codex 模型目录（`internal/extension/codex/catalog.go`）

为 `/v1/models` 生成 Codex CLI 期望的富模型元数据（`reasoning_levels`、`base_instructions`、`apply_patch_tool_type="freeform"`、`shell_type="unified_exec"`、`web_search_tool_type` 等），并能生成 Codex `config.toml`（`wire_api = "responses"`，model_provider 指向本网关）。`default_instructions.go` 内嵌默认 base_instructions 模板，按模型名替换 `{{MODEL_NAME}}`。

---

## 5. 对 Tauri/Rust 实现的预判差异点（供后续校对）

1. **两段式 vs 直转**：moon-bridge 经 Core 中间格式做 Responses↔Chat；若 tauri-gateway 选择 Responses→Chat **直转**，需自行确保上面合并映射表的每条规则都覆盖到（尤其 input 数组的 reasoning/tool_use 相邻性、连续 function_call 批处理）。
2. **`max_completion_tokens` 而非 `max_tokens`**：Chat 请求 DTO 字段名是 `max_completion_tokens`（chat/types.go:17）。Rust 端若沿用旧 `max_tokens`，对接较新 OpenAI/兼容上游可能被忽略。
3. **tool_call arguments 的字符串编码**：Chat 协议 `function.arguments` 始终是 JSON **字符串**（双重编码），入站需去引号、出站需包引号。Rust serde 实现这里极易踩坑（少一层/多一层引号）。
4. **`reasoning.effort → reasoning_effort` 透传**：易被遗漏；Responses 的 reasoning 对象在 Chat 里只剩一个 `reasoning_effort` 字符串。
5. **流式 sequence_number 与 item_id 规则**：Responses SSE 要求全局递增 `sequence_number` 及一整套 added/delta/done 事件配对，item_id 命名有固定前缀（msg_/rs_/fc_）。Rust 端若简化事件序列，codex CLI 可能解析失败。
6. **`stop` 字段疑似未映射**：moon-bridge 的 `ToCoreRequest` 未把 Responses `stop` 写入 Core——这可能是 moon-bridge 的遗漏而非规范，Rust 实现应主动决定是否映射 `stop→stop`。
7. **`parallel_tool_calls` 进了扩展但 Chat FromCoreRequest 未回写**：同样疑似遗漏，校对时确认 tauri-gateway 是否需要透传。
8. **codex apply_patch 代理是可选增强**：是否实现 apply_patch/exec 结构化代理直接影响 codex CLI 编辑文件能力；MVP 可先 ToolRaw 透传原始 grammar，但需保证 `custom_tool_call` 的 input 原样往返。
9. **透传判断维度**：moon-bridge 按 **provider 协议**（openai-response）决定透传，而非按入站路径；tauri-gateway 若用"端点类型=codex"作判断维度，语义需对齐——本质是"上游是否原生 Responses 协议"。
10. **无独立 codex 端点**：codex 复用 `/v1/responses` + `/v1/models`；tauri-gateway 新增"codex 端点类型"应理解为一种 provider 协议/上游形态，而非新增 URL 路由。
