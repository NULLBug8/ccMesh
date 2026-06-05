# 阶段 2：API 格式转换

> 任务总线：[../../TASKS.md](../../TASKS.md) ｜ 规划索引：[../README.md](../README.md)
> 起点参考：旧版 ccNexus 源码 `docs/origin/ccNexus/`（Go + Wails，仅参考不复用）

## 阶段目标

实现 Claude ↔ OpenAI Chat 双向格式转换：trait 抽象 + 注册表、非流式转换、工具调用（tool_use）、思考/推理内容、流式 SSE 增量与 usage 提取，并接入代理转发链路。与阶段 1 共同交付 **里程碑 M2 代理可用**。

## 前置依赖

- 阶段 0（`AppError`）、阶段 1（代理转发 P1-4、服务 P1-5、流式 P2-5 与 `streaming.rs` 共用）。

## 任务清单

### P2-1 Transformer trait 与注册表
- 所属层：Rust
- 文件：`src-tauri/src/modules/transform/transformer.rs`、`mod.rs`、`src-tauri/src/models/transform.rs`
- 实现要点：定义 `Transformer` trait（`transform_request`、`transform_response`、`transform_stream_chunk`）；`Registry` 按名称（`claude`/`openai`）注册与查找，配置驱动（端点 `transformer` 字段选择）。保留扩展位（后续 OpenAI Responses / Gemini）。
- 前置：P0-2
- 验收：注册/查找单测通过；未知 transformer 返回 `AppError::Transform`。
- PRD Story：11, 12, 扩展性

### P2-2 Claude ↔ OpenAI Chat 非流式转换
- 所属层：Rust
- 文件：`src-tauri/src/modules/transform/claude_openai.rs`、`types.rs`
- 实现要点：实现 `ClaudeReqToOpenAI` 与 `OpenAIToClaudeResp` 双向；映射 system/messages/role/content、`max_tokens`、`temperature` 等；保留 tool 定义与调用结构占位。参考旧版 `claude_openai.go`。
- 前置：P2-1
- 验收：给定 Claude 请求样例转 OpenAI 后字段正确；响应反向转换正确（单测）。
- PRD Story：11, 12, 13

### P2-3 工具调用（tool_use）转换
- 所属层：Rust
- 文件：`src-tauri/src/modules/transform/claude_openai.rs`
- 实现要点：Claude `tool_use` / `tool_result` 与 OpenAI `tool_calls` / `role:tool` 双向映射，含 `tool_call_id`、`arguments`（JSON 字符串 ↔ 对象）处理。
- 前置：P2-2
- 验收：含工具调用的请求/响应往返转换单测通过。
- PRD Story：14

### P2-4 思考/推理内容转换
- 所属层：Rust
- 文件：`src-tauri/src/modules/transform/claude_openai.rs`
- 实现要点：Claude `thinking` 块与 OpenAI `reasoning` / `reasoning_content` 双向映射；保留签名/隐藏字段策略。
- 前置：P2-2
- 验收：含 reasoning 的样例往返转换单测通过。
- PRD Story：15

### P2-5 流式 SSE 转换
- 所属层：Rust
- 文件：`src-tauri/src/modules/proxy/streaming.rs`、`src-tauri/src/modules/transform/claude_openai.rs`
- 实现要点：解析上游 SSE 流，逐 chunk 转换（OpenAI delta ↔ Claude `message_start/content_block_delta/message_delta/message_stop`），含工具调用与 reasoning 的流式增量；从流中提取 usage（input/output tokens）回传统计。参考旧版 `streaming.go` 与 `streaming_usage_test.go`。
- 前置：P2-3, P2-4, P1-4
- 验收：流式样例转换单测通过；usage 提取正确。
- PRD Story：13, 14, 15, 22, 23

### P2-6 代理接入转换器
- 所属层：Rust
- 文件：`src-tauri/src/modules/proxy/forward.rs`（接入点）
- 实现要点：根据客户端格式（`ClientFormat::Claude / OpenAIChat`）与端点 `transformer` 字段，在转发前后调用对应 Transformer；非流式与流式分别走 P2-2/P2-5。
- 前置：P2-5, P1-5
- 验收：端到端：Claude 客户端 → OpenAI 后端往返成功（集成测试）。
- PRD Story：11, 12, 13, 14, 15

## 参考材料（origin）

> 路径相对 `docs/origin/ccNexus/`

| 关注点 | 文件 | 关键符号 / 说明 |
|--------|------|-----------------|
| Transformer 接口 | `internal/transformer/transformer.go` | trait 抽象对标 |
| 请求/响应类型 | `internal/transformer/types.go` | `OpenAIRequest`、`ClaudeRequest` |
| 注册表 | `internal/transformer/registry.go` | 按名称注册/查找 |
| Claude ↔ OpenAI 转换 | `internal/transformer/convert/claude_openai.go`（+ `claude_openai_test.go`） | `ClaudeReqToOpenAI`、`OpenAIToClaudeResp` |
| 公共转换工具 | `internal/transformer/convert/common.go` | 角色/内容映射 |
| 工具链（tool_use） | `internal/transformer/tool_chain.go` | tool_calls 链路 |
| reasoning / think | `internal/transformer/convert/think_tags.go` | thinking ↔ reasoning |
| 流式 + usage 提取 | `internal/proxy/streaming.go`（+ `streaming_usage_test.go`） | SSE 增量、usage 抽取 |
| 转换器选择 | `internal/proxy/request.go` | `prepareTransformerForClient` |

> 进阶参考（本期只做 Claude↔OpenAI Chat，其余仅作扩展性设计参考）：`internal/transformer/cc/*`、`cx/chat/*`、`cx/responses/*`、`convert/{claude_gemini,openai_gemini,...}.go`。

## 完成判据（里程碑 M2，与阶段 1 合并）

- 非流式 / tool_use / reasoning / 流式 四类转换单测通过（详见 P10-1）；
- 端到端集成：Claude 客户端经代理打到 OpenAI 兼容后端，往返成功且 usage 正确入统计。
