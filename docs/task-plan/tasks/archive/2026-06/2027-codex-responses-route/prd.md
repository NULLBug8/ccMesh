# codex 端点与 /v1/responses 路由转发

## Goal

为网关新增 **codex 端点类型**，并实现 **`/v1/responses` 路由**：让只支持 OpenAI Responses API 的客户端（codex CLI）能通过本网关对接两类上游——原生 Responses 上游（**透传**）与仅支持 Chat Completions 的上游（**Responses↔Chat 双向协议转换**），并复用现有的代理开关、模型映射、轮换/熔断/统计能力。**不做 Responses↔Claude 方向的支持。**

## Requirements

1. **端点类型新增 codex**
   - 端点 `transformer` 字段新增取值 `"codex"`，语义 = 「上游为 OpenAI Responses API」。
   - codex 端点获取模型资源（`/v1/models`）的方式与现有 OpenAI 端点一致（Bearer + codex UA），复用既有逻辑。
2. **新增入站路由 `/v1/responses`（兼容 `/responses`）**
   - 接收 OpenAI Responses API 请求；按**选中的上游端点 transformer** 决定处理：
     - 选中 **codex 端点** → Responses **透传**（仅按映射改写 `model`，其余原样转发，流式字节直转）。
     - 选中 **openai 端点** → Responses → Chat Completions **请求转换**；上游返回 Chat 响应 → 转回 Responses **响应**（含流式 SSE 事件转换）。
   - 候选端点过滤：`/v1/responses` 入站时，候选 = **codex + openai** 两类端点；无可用候选返回 400。
3. **复用现有能力（零改动）**：代理开关（端点 `use_proxy` 或全局 `proxy_enabled`）、模型映射（入站模型名→出站模型名）、轮换/熔断/统计/请求日志。
4. **流式支持**：透传与转换两条路径均支持 `stream=true` 与非流式。
5. **前端**：端点类型下拉新增 codex 选项。

## Acceptance Criteria

- [ ] 端点 `transformer` 可设为 `"codex"`，创建/读取/更新往返正确。
- [ ] codex 端点能成功拉取 `/v1/models` 模型列表（走 OpenAI Bearer + codex UA 分支）。
- [ ] `POST /v1/responses` 选中 codex 端点：请求原样透传到上游 `/v1/responses`，仅 `model` 按映射改写；响应原样回传；流式字节直转。
- [ ] `POST /v1/responses` 选中 openai 端点（非流式）：Responses 请求正确转为 Chat 请求（`input→messages`、`instructions→system`、`max_output_tokens→max_completion_tokens`、`tools`/`tool_choice`/`reasoning.effort` 映射）；上游 Chat 响应正确转为 Responses 响应（`output`/`output_text`、`usage.input_tokens`/`output_tokens`、`status`）。
- [ ] `POST /v1/responses` 选中 openai 端点（流式）：上游 Chat SSE 正确转为 Responses SSE 事件序列（`response.created → output_item.added → content_part.added → output_text.delta(*) → output_text.done → content_part.done → output_item.done → response.completed`），带全局递增 `sequence_number` 与正确 `item_id` 前缀（`msg_`/`rs_`/`fc_`）。
- [ ] 代理开关与模型映射在 codex / openai 两条路径均生效。
- [ ] 前端端点表单可选择 codex 类型并保存。

## Definition of Done

- `cargo check` / `cargo test` 通过；新增转换逻辑有单元测试（请求映射、响应映射、流式事件序列、透传）。
- 前端类型检查通过。
- 字段映射与 moon-bridge 调研（research/03）逐条校对，差异点（`stop` / `parallel_tool_calls`）有明确决策并落实。
- `progress.csv` 子任务全部标记完成。

## User Stories

- 作为使用 codex CLI 的开发者，我希望把网关配成 codex 的 `model_provider`，以便用任意 Chat Completions 上游（DeepSeek / 本地 / 第三方中转）驱动 codex。
- 作为使用原生 Responses 上游（如 aimlapi codex）的用户，我希望网关透传 Responses 请求，以便享受网关的代理/轮换/熔断/统计而不损失协议特性。
- 作为网关用户，我希望 codex 端点也能配模型映射与代理开关，体验与其他端点一致。

## Implementation Decisions

- **端点类型表示**：沿用 `transformer` 字符串新增 `"codex"`，**不新增数据库列、不迁移**；运行期 `UpstreamFormat` 新增 `OpenAiResponses` 变体，`from_transformer_name("codex") → OpenAiResponses`。
- **转换/透传判定维度** = 上游端点 transformer（codex=透传，openai=转换），本质是「上游是否原生 Responses 协议」，对齐 moon-bridge 的 provider 协议判断。
- **路由挂载**：`/v1/responses` 走现有 fallback 代理处理器 + 路径识别（`path` 含 `/responses`），**不加显式 route**，复用轮换/熔断/统计/代理全套。
- **转换策略**：Responses↔Chat 采用**直转**（不引入中间格式），但字段规则逐条对齐 moon-bridge 合并映射表。
- **关键字段约定**：
  - `max_output_tokens → max_completion_tokens`（**不是** `max_tokens`）。
  - `reasoning.effort → reasoning_effort`（Chat 顶层）。
  - `instructions` → 单条 `role:"system"` 消息，置于 messages 最前。
  - tool_call `arguments` 在 Chat 侧**始终为 JSON 字符串**（双重编码）：入站去引号、出站包引号。
  - `stop`：Responses 文档未列为入参，moon-bridge 亦未映射 → **决策：不映射**（透传时原样保留，转换时丢弃）。
  - `parallel_tool_calls`：转换时**透传**到 Chat（修正 moon-bridge 疑似遗漏）。
- **流式 Responses SSE**：实现完整事件序列 + 全局 `sequence_number` + `item_id` 前缀，对齐 codex CLI 预期，不做简化。
- **鉴权 / UA**：codex → `Authorization: Bearer`（同 openai）；UA 复用 openai 伪装 UA。
- **模型获取**：`OpenAiResponses` 落入现有 OpenAI 鉴权分支，零改动复用（codex probe UA 已在用）。

## Testing Decisions

- 单元测试覆盖：请求映射（string input / array input / instructions / tools / tool_choice / reasoning / max_output_tokens）、响应映射（text / tool_calls / usage / finish_reason→status）、透传（仅改 model）、流式事件序列（断言事件类型顺序、`sequence_number` 递增、`item_id` 前缀）。
- 与 moon-bridge research/03 的映射表逐条人工对照，记录差异决策。
- 真实 codex CLI 端到端连接属 GUI/外部网络行为，**无法在无头环境自动验证**，需本地手动核对（见 feature.md 核对清单）。

## Out of Scope

- Responses ↔ Claude 协议转换。
- codex `apply_patch` / `exec` 结构化工具语法代理（grammar 重建）—— MVP 先按 function/custom 工具**原样往返**（不破坏 input/arguments），可作后续增强。
- `previous_response_id` 服务端会话续接 / `store` 存储 —— 透传交上游；转换路径不实现服务端存储。
- `/v1/models` 的 codex 富元数据目录（`reasoning_levels` / `base_instructions` 等 moon-bridge catalog）—— 复用现有 `/v1/models`（返回 id 列表）即可。

## Technical Notes

- **EchoBird 路线已放弃**：其 Responses↔Chat 转换核心位于私有 crate `echobird_core`（`git = https://github.com/edison7009/EchoBird-secret-.git`），公开仓库任何分支/tag 仅含 `tools/codex/README.md`、`config.json` 等配置与文档，**无转换源码**。故采用**自研 + moon-bridge(Go) 逐字段校对**。
- codex 官方（gpt-5.3-codex）仅文档化 Responses API，Chat Completions 被定位为 legacy 且无 schema（research/04）。「Responses→Chat」转换的价值在于让 codex 客户端能驱动**仅支持 Chat Completions 的第三方上游**。
- 调研依据：research/01（本项目落点）、research/03（moon-bridge 映射基准）、research/04（codex 参数）。
