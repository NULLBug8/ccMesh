# 现有架构调研：codex 端点类型 + /v1/responses 路由

> 调研范围：为新增 codex 端点类型、新增 `/v1/responses` 路由（Responses API → Chat Completions 转换 + Responses → Responses 透传）摸清现有落点。
> 结论一句话：**端点类型用 `transformer` 字符串字段表示，无需新增枚举/数据库列；codex 端点存量即可承载（仅 transformer 取值新增 codex）；核心工作集中在路由挂载 + 入站格式识别 + 新增 Responses↔Chat 转换模块。**

---

## 1. 端点类型定义

### 1.1 数据结构
- 文件：`src-tauri/src/models/endpoint.rs`
  - `struct Endpoint`（`endpoint.rs:14`）核心字段：
    - `transformer: String`（`endpoint.rs:25`）—— **端点类型即由此字符串表示**，注释写「转换器名：claude / openai」。
    - `model: String`（`endpoint.rs:27`）：锁定模型（非空则强制覆盖）。
    - `models: Vec<String>`（`endpoint.rs:29`）：对外公布模型清单。
    - `model_mappings: Vec<ModelMapping>`（`endpoint.rs:31`）：入站→出站模型映射（`ModelMapping{from,to}`，`endpoint.rs:6`）。
    - `use_proxy: bool`（`endpoint.rs:23`）：端点级代理开关。
  - `CreateEndpointRequest`（`endpoint.rs:42`）/ `UpdateEndpointRequest`（`endpoint.rs:67`）同样以字符串携带 `transformer`，默认值 `default_transformer() -> "claude"`（`endpoint.rs:87`）。

### 1.2 类型表示方式 —— 字符串，不是枚举
- **端点本身不用枚举**，只存字符串 `transformer`。
- 运行期统一收敛到枚举 `UpstreamFormat`：`src-tauri/src/modules/transform/transformer.rs:5`
  ```rust
  pub enum UpstreamFormat { Claude, OpenAiChat }
  pub fn from_transformer_name(name: &str) -> Self {  // transformer.rs:13
      match name.trim().to_ascii_lowercase().as_str() {
          "openai" | "openai_chat" | "openai-chat" | "openai2" => UpstreamFormat::OpenAiChat,
          _ => UpstreamFormat::Claude,  // ← 未知（含 codex）当前全部当 Claude 直通
      }
  }
  ```
  **关键**：该函数注释（`transformer.rs:12`）已明确写「gemini / openai_responses / **codex** 等在本项目 Out of Scope，按 Claude 直通处理」。即 codex 目前会被错误地当成 Claude 直通。

### 1.3 transformer 如何影响三条链路
1. **路由匹配 / 候选过滤**：`forward.rs:246-266`。当前仅识别 `/chat/completions` 为 OpenAI 入站，并用 `UpstreamFormat::from_transformer_name` 过滤出 `OpenAiChat` 端点。
2. **请求转发的转换决策**：`forward.rs:326-371`。`needs_transform = !inbound_openai && OpenAiChat` 时走 Claude→OpenAI 转换；上游路径改写为 `/v1/chat/completions`。
3. **鉴权头**：`forward.rs:598-607`，`match ep.transformer.as_str() { "openai"|"openai2"|"openai_chat" => Bearer, _ => x-api-key + Bearer }`。
4. **伪装 UA**：`forward.rs:563-567`，按 `UpstreamFormat` 选 `openai_ua` / `claude_cli_ua`。
5. **模型获取**：`models_cache.rs:34-45`，按 `UpstreamFormat` 决定鉴权与 UA（OpenAI 分支已带 codex UA/originator，见 §5）。

### 1.4 新增 codex 类型需改动的位置（清单）
- `transformer.rs:13-18`：`from_transformer_name` 增加 `"codex"` 分支（映射到现有 `OpenAiChat`，或新增枚举值 `OpenAiResponses`/`Codex` —— 取决于上游协议差异，见 §4）。
- `forward.rs:598-607`：鉴权头 `match` 增加 `"codex"` → Bearer（codex 走 OpenAI 鉴权）。
- `forward.rs:563-567`：UA 分支若新增枚举需补齐。
- 路由层与入站识别（§2）。
- 前端 Select 选项（§7）。
- **无需新增数据库列/迁移**（§6）。

---

## 2. 路由层

- 文件：`src-tauri/src/modules/proxy/server.rs`
- 路由构建：`fn build_router`（`server.rs:68-76`）：
  ```rust
  Router::new()
      .route("/health", get(health_route))
      .route("/stats", get(stats_route))
      .route("/v1/models", get(models_route))                       // server.rs:72
      .route("/v1/messages/count_tokens", post(count_tokens_route)) // server.rs:73
      .fallback(handle_proxy)                                        // server.rs:74 ← 兜底
      .with_state(state)
  ```
- **现有暴露路由**：`/health`、`/stats`、`/v1/models`(GET)、`/v1/messages/count_tokens`(POST)；其余一切（含 `/v1/messages`、`/v1/chat/completions`）走 **fallback `handle_proxy`**（`forward.rs:185`）。即 `/v1/messages`、`/v1/chat/completions` 没有显式 route，靠 fallback + 路径字符串识别。
- **`/v1/responses` 挂载方式**：两种可选
  - (A) 不加显式 route，直接在 `handle_proxy` 内按 `path.contains("/responses")` 识别（与现有 `/chat/completions` 识别一致，最小改动）。
  - (B) 加显式 `.route("/v1/responses", post(...))`，单独 handler；适合需要独立转换流程时。
  - 推荐 (A)：复用 `handle_proxy` 的轮换/熔断/统计/代理全套能力，仅在 §1.3-1 的入站识别处增加 `/responses` 分支。

---

## 3. 请求转发

- 文件：`src-tauri/src/modules/proxy/forward.rs`
- 主入口：`pub async fn handle_proxy`（`forward.rs:185`）。流程：解析 body → 端点候选过滤 → `resolver::resolve_endpoint` → 轮换/熔断选路 → `send_upstream` → 按 (needs_transform, stream) 四象限处理响应（`forward.rs:420-425`）。
- 上游发送：`async fn send_upstream`（`forward.rs:531`），`url = api_url + upstream_path`（`forward.rs:539-540`）。

### 3.1 「代理开关」判断位置
- `forward.rs:544-559`：`let want_proxy = ep.use_proxy || st.proxy_enabled;`（端点级 `use_proxy` 或全局 `proxy_enabled`，二选一为真即走 `proxy_client`，无可用代理则 warn 回落直连）。
- 全局开关 `proxy_enabled` 来自 `ProxyState`（`forward.rs:85`），`start_proxy` 时读配置注入（`server.rs:127`）。
- 模型拉取侧同款逻辑：`should_use_proxy(ep.use_proxy, proxy_enabled, &proxy_url)`（`commands/models.rs:55`、`87`）。
- **结论**：codex 端点复用 `use_proxy` 字段，**无需改动代理开关逻辑**。

### 3.2 「模型映射」应用位置
- 文件：`src-tauri/src/modules/proxy/resolver.rs`
  - `resolve_outbound(ep, inbound)`（`resolver.rs:119`）：① 入站名命中 `model_mappings[].from`（大小写不敏感）→ 返回 `.to`；② 否则锁定 `model` 非空 → 返回锁定值；③ 否则 `None`（透传）。
  - `advertised_models(ep)`（`resolver.rs:95`）：对外公布集合 = 基础模型(锁定 model 优先，否则 models) ∪ 所有映射入站名。供 `/v1/models` 与路由匹配。
  - `filter_by_model(enabled, model)`（`resolver.rs:141`）：按请求模型过滤候选端点（故障隔离）。
- 转发侧调用：`forward.rs:328` `let outbound_model = resolver::resolve_outbound(&ep, model.as_deref())`；随后 §1.3-2 用 outbound_model 覆盖请求 model。
- **结论**：codex 端点复用 `resolve_outbound`/`advertised_models`/`filter_by_model`，**无需改动映射逻辑**；只要 Responses 请求体能取到/写回 `model` 字段即可（Responses API 的 `model` 字段与 Chat 一致，在 body 顶层）。

---

## 4. 协议转换（transform 模块）

- 模块目录：`src-tauri/src/modules/transform/`，`mod.rs` 暴露 6 个子模块：
  `claude_openai`、`json_canonical`、`streaming`、`thinking_rectifier`、`transformer`、`types`。

### 4.1 现有转换是 Claude ↔ OpenAI Chat
- `claude_openai.rs` **是 Claude Messages ↔ OpenAI Chat Completions**（不是 Responses）：
  - 请求：`pub fn claude_request_to_openai(claude: &Value, endpoint_model: Option<&str>) -> Value`（`claude_openai.rs:18`）。
  - 非流式响应：`pub fn openai_response_to_claude(resp: &Value) -> Value`（`claude_openai.rs:256`）。
  - trait：`Transformer::transform_request`（`transformer.rs:22-29`）；`ClaudeOpenAiTransformer`（`claude_openai.rs:9`）；`IdentityTransformer`（`transformer.rs:32`，Claude 直通）；分发 `get_transformer(format)`（`transformer.rs:45`）。
- 流式：`streaming.rs` 的 `struct StreamConverter`（`streaming.rs:9`）—— 有状态消费 OpenAI Chat SSE chunk，产出 Claude SSE 事件。核心：`new(model, input_tokens)`（`streaming.rs:16`）、`process_chunk(&Value) -> Vec<String>`（`streaming.rs:204`）、`finish() -> Vec<String>`（`streaming.rs:301`）、`usage()`（`streaming.rs:28`）。
- 共享类型/工具：`types.rs`
  - `StreamContext`（`types.rs:15`）、`ToolBlock`（`types.rs:8`）：流式累积状态。
  - `extract_tool_result_content`（`types.rs:34`）、`map_finish_reason`（`types.rs:48`）、`build_claude_event`（`types.rs:61`，生成 `event:\ndata:\n\n` SSE 文本）。
- `json_canonical.rs`：`canonical_json_string`（被 `claude_openai.rs:136`、`types.rs:43` 用于工具参数键排序，转换 tool 调用时复用）。

### 4.2 新增「Responses API ↔ Chat Completions」模块放置与复用
- **放置**：新增 `src-tauri/src/modules/transform/responses_chat.rs`（或 `codex_responses.rs`），在 `mod.rs` 追加 `pub mod responses_chat;`（`mod.rs:1-6` 处）。
- **方向**（与现有「客户端固定 Claude」模型不同，这里入站是 Responses）：
  - 透传：Responses → Responses（codex 端点上游本身就是 Responses 时，IdentityTransformer 思路，仅 model 改写）。
  - 转换：Responses → Chat Completions（codex 端点上游是 Chat 时）；以及响应方向 Chat → Responses。
- **可复用**：
  - `json_canonical::canonical_json_string`（工具参数序列化）。
  - `types::{map_finish_reason, build_claude_event}` 的同构思路（build_claude_event 是 Claude 专用，Responses SSE 需新写 `build_responses_event`，但 SSE 拼装模式可照搬）。
  - `StreamConverter` 的有状态分片消费模式可作为模板（需为 Responses 事件模型新写一个 converter）。
  - `usage.rs`（`from_response` / `UsageAccumulator`，见 `forward.rs:22,28`）—— 需为 Responses usage 字段扩展或新增分支。
- **接入点**：`transformer.rs` 的 `UpstreamFormat` 与 `get_transformer` 需扩展（新增 `OpenAiResponses` 变体或 codex 专用分支），`forward.rs:420-425` 四象限响应处理需增加 Responses 的转换/透传分支。

---

## 5. 模型资源获取

- 命令层：`src-tauri/src/commands/models.rs`
  - `get_models`（`models.rs:15`）：带缓存的全量拉取，遍历启用端点调用 `fetch_models`。
  - `fetch_endpoint_models`（`models.rs:75`）：表单「刷新」按钮，按字段（api_url/api_key/transformer/use_proxy）拉取，端点未保存也可用。
- 实现层：`src-tauri/src/modules/models_cache.rs`
  - `fetch_model_ids(client, api_url, api_key, transformer)`（`models_cache.rs:26`）：
    - 按 `UpstreamFormat::from_transformer_name(transformer)` 分流（`models_cache.rs:34`）。
    - **OpenAI 分支已自带 codex 客户端伪装**（`models_cache.rs:35-39`）：
      ```rust
      .header("user-agent", crate::utils::ua::codex_probe_ua())   // ua.rs:16
      .header("originator", crate::utils::ua::CODEX_ORIGINATOR)    // ua.rs:13 = "codex_cli_rs"
      .header("authorization", format!("Bearer {api_key}"))
      ```
    - 两种上游 `/v1/models` 响应都解析 `data[].id`（`models_cache.rs:49-54`）。
  - `fetch_models`（`models_cache.rs:62`）：失败回落默认模型 `model_info(default_model(ep), ...)`。
- **codex 复用方式**：codex 端点拉模型走 `/v1/models` + Bearer + codex UA，与现有 OpenAI 分支完全一致。
  - 若 `from_transformer_name("codex")` 映射到 `OpenAiChat`/或新枚举的 OpenAI 系，则**模型获取零改动即复用**；只需保证 codex 落入 OpenAI 鉴权分支（`models_cache.rs:35`）。

---

## 6. 数据库迁移

- 文件：`src-tauri/src/modules/storage/migration.rs`
- 当前最高版本：**v8**（`MIGRATIONS` 共 8 条，`migration.rs:6-122`；末条 v8 `migration.rs:120-121` 给 request_logs 加 `actual_model`）。
- 端点类型存储：v1 建表时 `transformer TEXT NOT NULL DEFAULT 'claude'`（`migration.rs:15`）—— **就是个文本列，存任意字符串**。后续 v2 加 `models`/`use_proxy`（`migration.rs:66-67`），v7 加 `model_mappings`（`migration.rs:119`）。
- 仓储读写：`src-tauri/src/modules/storage/endpoint_repo.rs`，`COLS` 含 `transformer`（`endpoint_repo.rs:6`），`from_row`（`endpoint_repo.rs:17`）/`create`（`endpoint_repo.rs:64`）/`update`（`endpoint_repo.rs:106,135`）原样透传字符串。
- **结论**：codex 只是 `transformer` 列的一个新取值（`"codex"`），**无需新增迁移版本、无需新列**。除非 codex 需要额外配置字段（如独立的 responses base path），届时才追加 v9。

---

## 7. 前端端点类型

- 框架：**React + TSX**（不是 Vue）。
- 类型选项定义：`src/pages/Endpoints/_components/EndpointForm.tsx:205-212`
  ```tsx
  <Select value={form.transformer} onValueChange={(v) => set("transformer", v)}>
    ...
    <SelectItem value="claude">claude（直通）</SelectItem>   // 行 210
    <SelectItem value="openai">openai（转换）</SelectItem>   // 行 211
  </Select>
  ```
  默认值 `transformer: "claude"`（`EndpointForm.tsx:46`）。
- 类型声明：`src/services/modules/endpoint.ts`，`Endpoint.transformer: string`（`endpoint.ts:17`）、`CreateEndpointInput.transformer?: string`（`endpoint.ts:35`）；`fetchModels(apiUrl, apiKey, transformer, useProxy)`（`endpoint.ts:88-99`）传给后端 `fetch_endpoint_models`。
- **新增 codex 选项**：仅需在 `EndpointForm.tsx:211` 后追加一条 `<SelectItem value="codex">codex（Responses）</SelectItem>`。`transformer` 是自由字符串，类型声明无需改动。

---

## 8. 改动落点清单（汇总）

### 后端
| 文件 | 位置 | 改动 |
|---|---|---|
| `transform/transformer.rs` | `:13` from_transformer_name；`:5` UpstreamFormat；`:45` get_transformer | 新增 codex 识别；视协议差异决定是否新增枚举变体（OpenAiResponses/Codex） |
| `transform/mod.rs` | `:1-6` | `pub mod responses_chat;`（新模块） |
| `transform/responses_chat.rs` | 新文件 | Responses↔Chat 转换 + Responses 透传；复用 json_canonical / types / StreamConverter 模式 |
| `proxy/forward.rs` | `:246-266` 入站识别 | 增加 `/v1/responses` 入站分支 + codex 端点候选过滤 |
| `proxy/forward.rs` | `:326-371` 转换决策 / `:420-425` 响应四象限 | 增加 Responses 转换/透传与上游路径改写 |
| `proxy/forward.rs` | `:598-607` 鉴权头 | `"codex"` → Bearer |
| `proxy/forward.rs` | `:563-567` 伪装 UA | 若新增枚举需补齐 UA 分支 |
| `proxy/server.rs` | `:68-76` | （可选）显式挂 `/v1/responses` route；推荐走 fallback |
| `models_cache.rs` | `:34` | 确保 codex 落入 OpenAI 鉴权分支（多半零改动） |
| `usage.rs` | `from_response`/`UsageAccumulator` | 视需要扩展 Responses usage 字段 |

### 前端
| 文件 | 位置 | 改动 |
|---|---|---|
| `src/pages/Endpoints/_components/EndpointForm.tsx` | `:211` | 追加 `<SelectItem value="codex">` |

### 数据库
- **无需迁移**（transformer 为文本列，复用 use_proxy / model_mappings / models）。除非 codex 需新增独立配置字段，才追加 v9。

### 关键复用结论
- 代理开关：复用 `ep.use_proxy || proxy_enabled`（forward.rs:545 / models.rs:55），零改动。
- 模型映射：复用 `resolver::resolve_outbound`（resolver.rs:119），零改动。
- 模型获取：复用 `fetch_model_ids` OpenAI 分支（models_cache.rs:34，已带 codex UA），近零改动。
