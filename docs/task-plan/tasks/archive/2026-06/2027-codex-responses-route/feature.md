# 2027 codex 端点与 /v1/responses 路由转发

## 目标

新增 codex 端点类型（`transformer="codex"`，语义=上游原生 Responses API）与 `/v1/responses` 入站路由：codex 端点走 **Responses 透传**，openai 端点走 **Responses↔Chat 双向转换**（含流式）。复用现有代理开关/模型映射/轮换/熔断/统计，零改动。

## 现状（根因）

- 端点类型即 `Endpoint.transformer: String`（`models/endpoint.rs:25`），运行期收敛枚举 `UpstreamFormat{Claude, OpenAiChat}`（`transform/transformer.rs:5,13`）。**codex 现被 `from_transformer_name` 落入 `_ => Claude`，错误地当 Claude 直通**。
- 入站格式仅识别 `/chat/completions`（`proxy/forward.rs:246` `inbound_openai`），**无 `/responses` 分支**。
- transform 层只有 Claude↔OpenAI Chat（`transform/claude_openai.rs`、`transform/streaming.rs`），**无 Responses 方向**。
- 响应处理是 `(needs_transform, stream)` 四象限（`forward.rs:420-425`）。
- 数据库 v8，`transformer` 为文本列（`storage/migration.rs:15`），**无需迁移**；`use_proxy`/`model_mappings`/`models` 均可复用。

## 关键文件/落点

### 后端 — 纯逻辑层（transform）
| 文件 | 落点 | 改动 |
|---|---|---|
| `src-tauri/src/modules/transform/transformer.rs` | `:5` enum / `:13` from_transformer_name / `:45` get_transformer | `UpstreamFormat` 新增 `OpenAiResponses`；`"codex" => OpenAiResponses`；`get_transformer` 中 OpenAiResponses 暂用 Identity 占位（实际转换在 responses_chat，不走该 trait） |
| `src-tauri/src/modules/transform/mod.rs` | `:1-6` | 追加 `pub mod responses_chat;` |
| `src-tauri/src/modules/transform/responses_chat.rs` | **新文件** | Responses→Chat 请求转换、Chat→Responses 响应转换（非流式）、`ResponsesStreamConverter`（流式）、透传辅助（改写 model）。复用 `json_canonical::canonical_json_string`、`types::map_finish_reason` |
| `src-tauri/src/modules/transform/types.rs` | `:48,61` | 复用 `map_finish_reason`；新增 `build_responses_event(event_type, json)`（SSE 文本拼装，参照 `build_claude_event` 模式） |

### 后端 — 集成层（proxy）
| 文件 | 落点 | 改动 |
|---|---|---|
| `src-tauri/src/modules/proxy/forward.rs` | `:246-266` 入站识别+候选过滤 | 新增 `inbound_responses = path.contains("/responses")`；候选过滤为 `OpenAiResponses ∪ OpenAiChat` 端点，空则 400 |
| `src-tauri/src/modules/proxy/forward.rs` | `:326-371` 转换决策+上游路径 | 三态：codex(OpenAiResponses)→透传(`/v1/responses`)；openai(OpenAiChat)+responses入站→转换(请求 `responses_request_to_chat`，上游 `/v1/chat/completions`)；其余维持现状 |
| `src-tauri/src/modules/proxy/forward.rs` | `:420-425` 响应分支 | 增加 responses 维度：透传走现有 `relay_stream_response`/`relay_buffered_response`；转换走**新增** `stream_responses_from_chat` / `buffered_responses_from_chat` |
| `src-tauri/src/modules/proxy/forward.rs` | `:563-567` UA / `:598-607` 鉴权 | UA 分支补 `OpenAiResponses => openai_ua`；鉴权 match 补 `"codex" => Bearer` |
| `src-tauri/src/modules/proxy/forward.rs` | `:404` inbound_format 记录 | RequestMeta.inbound_format 增加 "responses" 取值 |
| `src-tauri/src/modules/models_cache.rs` | `:34` | 确保 `OpenAiResponses` 落入 OpenAI 鉴权分支（codex UA 已在用）——多半只需 match 补变体 |

### 前端
| 文件 | 落点 | 改动 |
|---|---|---|
| `src/pages/Endpoints/_components/EndpointForm.tsx` | `:211` | 追加 `<SelectItem value="codex">codex（Responses）</SelectItem>` |

## 任务拆解

- **2027.1** transformer 扩展：`UpstreamFormat::OpenAiResponses` + `from_transformer_name`/`get_transformer`/UA/鉴权识别 `codex`；确保 `models_cache` match 覆盖新变体。单测：`from_transformer_name("codex")==OpenAiResponses`。
- **2027.2** Responses→Chat **请求**转换 `responses_request_to_chat`。单测：string input、array input（system/user/assistant/tool 结果）、instructions 置首、tools、tool_choice(Raw)、reasoning.effort、max_output_tokens→max_completion_tokens、parallel_tool_calls 透传。
- **2027.3** Chat→Responses **响应**转换（非流式）`chat_response_to_responses`。单测：text→output_text、tool_calls→function_call(arguments 去引号)、usage 映射、finish_reason→status。
- **2027.4** Responses **流式**转换 `ResponsesStreamConverter`（消费 Chat SSE chunk，产出 Responses SSE）。单测：固定 chunk 序列断言事件类型顺序、`sequence_number` 递增、`item_id` 前缀（msg_/rs_/fc_）。
- **2027.5** forward.rs 集成：入站识别 + 候选过滤 + 三态决策 + 上游路径改写 + UA/鉴权 + 响应分支接入。
- **2027.6** 透传路径：codex 端点 Responses→Responses（仅按 `resolve_outbound` 改写 model，relay 复用，含流式字节直转）。
- **2027.7** 模型获取校验：codex 端点 `/v1/models` 复用 OpenAI 分支（Bearer + codex UA），拉取成功。
- **2027.8** 前端 codex 选项。
- **2027.9** 整体回归（`cargo check`/`cargo test`/前端 typecheck）+ 与 moon-bridge research/03 映射表逐条校对。

## 数据契约

### Responses 请求体（入站，关键字段）
```jsonc
{
  "model": "string",                  // 必填，经 resolve_outbound 改写
  "input": "string | array",          // 必填；string→单条 user；array→见下
  "instructions": "string?",          // → system 消息置首
  "max_output_tokens": "int?",        // → max_completion_tokens
  "tools": "array?",                  // function/web_search/mcp/local_shell
  "tool_choice": "string|object?",    // 保留 Raw
  "parallel_tool_calls": "bool?",     // 透传
  "reasoning": {"effort": "low|medium|high?"},  // → reasoning_effort
  "stream": "bool?"                   // 决定流式分支
}
// input array item.type: input_text/text/output_text→text; input_image→image;
//   function_call/custom_tool_call/local_shell_call→assistant tool_use(连续批处理同一assistant消息);
//   function_call_output/...→role:"tool"(带 call_id); reasoning→merge到紧随assistant前
```

### 请求字段映射（Responses → Chat，对齐 research/03 §2.1）
| Responses | Chat | 注意 |
|---|---|---|
| `input`(string) | `messages:[{role:user,content}]` | |
| `input`(array) | `messages[]` | 按 item 展开，见上 |
| `instructions` | `messages[0]` role:system（单条，置首） | |
| `max_output_tokens` | **`max_completion_tokens`** | 非 max_tokens |
| `tools[].{name,description,parameters}` | `tools[].{type:function,function:{...}}` | |
| `tool_choice` | `tool_choice` | 优先回写原始 Raw |
| `reasoning.effort` | `reasoning_effort`（顶层字符串） | |
| `parallel_tool_calls` | `parallel_tool_calls` | 透传（修正 moon-bridge 遗漏） |
| `stop` | （丢弃） | 决策不映射 |
| tool_call `arguments` | JSON **字符串**（双重编码） | 出站包引号 |

### 响应字段映射（Chat → Responses，对齐 research/03 §2.2）
| Chat | Responses | 注意 |
|---|---|---|
| `id` | `response.id` | |
| — | `response.object="response"` | 固定 |
| `choices[0].finish_reason` | `response.status` | length→incomplete；content_filter→failed；否则 completed |
| `choices[].message.content` | `output[]{type:message,content[]{type:output_text,text}}` + 顶层拼接 | |
| `choices[].message.tool_calls[]` | `output[]{type:function_call,arguments}` | arguments 去引号 |
| `usage.prompt_tokens` | `usage.input_tokens` | |
| `usage.completion_tokens` | `usage.output_tokens` | |
| `usage.total_tokens` | `usage.total_tokens` | |
| `usage.prompt_tokens_details.cached_tokens` | `usage.input_tokens_details.cached_tokens` | |

### 流式事件序列（Chat SSE → Responses SSE，对齐 research/03 §2.4）
全局递增 `sequence_number`；item_id 前缀 text=`msg_`、reasoning=`rs_`、tool=`fc_`：
```
response.created → response.in_progress
  [文本] response.output_item.added → response.content_part.added
         → response.output_text.delta(*) → response.output_text.done
         → response.content_part.done → response.output_item.done
  [推理] response.reasoning_summary_part.added → response.reasoning_summary_text.delta(*)
         → response.reasoning_summary_part.done
  [工具] response.output_item.added(function_call) → response.function_call_arguments.delta(*)
         → response.function_call_arguments.done → response.output_item.done
response.completed | response.incomplete | response.failed   // payload 内嵌完整 response 快照
```

### 透传（codex 端点）
- 上游 URL：`api_url` 去尾斜杠，未以 `/responses`/`/v1/responses` 结尾则补 `/v1/responses`。
- body：仅按 `resolve_outbound` 改写顶层 `model`，其余原样。
- 响应：relay 原始字节（流式/非流式），不解析 SSE。

## 验收标准

见 prd.md «Acceptance Criteria»（codex 保存往返、模型拉取、透传、非流式转换、流式转换、代理/映射生效、前端选项、空候选 400）。

## 测试点

- **单测**（`cargo test`，放各模块 `#[cfg(test)]`）：2027.1~2027.4 的映射/事件断言；透传仅改 model。
- **映射校对**：与 research/03 §2.1/§2.2/§2.4 逐条对照，差异点（stop 丢弃、parallel_tool_calls 透传）确认落实。
- **无头限制声明**：真实 codex CLI 端到端、真实上游出网、流式渲染**无法在无头环境验证**。本地核对清单：
  1. 配 codex 端点（指向 aimlapi 等原生 Responses 上游）→ `codex` CLI 设 `model_provider` 指向网关 → 验证透传可用。
  2. 配 openai 端点（指向 Chat-only 上游如 DeepSeek）→ codex CLI 连接 → 验证 Responses↔Chat 转换 + 流式输出正常。
  3. UI 端点表单选 codex、保存、刷新模型列表。

## 提交策略

按模块 scoped 提交（精确文件路径，绝不 `git add -A`）：
1. **docs**：`prd.md` `feature.md` `progress.csv` `research/*`（单独成提交）。
2. **后端纯逻辑+单测**：`transform/transformer.rs` `transform/mod.rs` `transform/responses_chat.rs` `transform/types.rs`（2027.1~2027.4）。
3. **后端集成**：`proxy/forward.rs` `modules/models_cache.rs`（2027.5~2027.7）。
4. **前端**：`src/pages/Endpoints/_components/EndpointForm.tsx`（2027.8）。
