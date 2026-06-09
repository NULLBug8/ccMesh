# 流式 message_delta 回传完整 usage（含 cache_read）

## Goal

Claude→OpenAI 流式转换时，把完整 token 用量（input/output/cache_read/cache_creation）回传给客户端，
修复客户端在流式下收不到 `cache_read_input_tokens`（及 `input_tokens` 恒 0）的问题。

## Requirements

- 流式 `message_delta`（[DONE] 时发）的 usage 携带 `input_tokens`/`output_tokens`/`cache_creation_input_tokens`/`cache_read_input_tokens`，取自流末已就绪的累积值。
- 不破坏现有流式行为（文本/工具/stop_reason/单工具与多工具）。

## Acceptance Criteria

- [ ] 流式：末尾 usage chunk 带 `prompt_tokens_details.cached_tokens` 时，最终 message_delta 的 usage 含正确的 `input_tokens` 与 `cache_read_input_tokens`。
- [ ] 无 usage 时 message_delta usage 各字段为 0（不报错）。
- [ ] 现有 streaming 测试不回归；`cargo test` 通过。

## Definition of Done

实现 + 单测；进度写回；scoped 提交；真实流式缓存命中需真实后端验证（显式声明）。

## User Stories

- 作为经本网关流式对接 OpenAI 兼容后端的用户，我希望客户端在流式响应里能看到缓存读取与输入 token，以便缓存与成本显示正确。

## Implementation Decisions

- **在最终 message_delta 回传完整 usage**（对齐 cc-switch）：message_delta 已延迟到 `[DONE]` 发，此时累积 usage 已就绪。
- message_start 维持现状（OpenAI 通常首块无 usage，保持 0；真实值由 message_delta 补齐）——与 cc-switch 一致，避免为等 usage 而阻塞首事件。
- 四字段恒发（含 0），与 message_start 结构一致，简化客户端解析。

## Testing Decisions

- 新增流式单测：content + 末尾 usage-only chunk（带 cached_tokens）→ 断言 message_delta 含 input_tokens/output_tokens/cache_read_input_tokens。
- 真实出网缓存命中无法无头验证。

## Out of Scope

- message_start 的 usage 改造（不阻塞等待 usage）。
- 非流式路径（`openai_response_to_claude` 已正确读取，无需改）。
- 网关自身统计（`converter.usage()` 已正确，无需改）。

## Technical Notes

- 落点：`src-tauri/src/modules/transform/streaming.rs::finish`（:308-314 的 message_delta usage）。
- 参考与根因见 research/streaming-usage.md。

## 任务拆解

- **2027.1 [集成] finish 回传完整 usage** —— 改 `streaming.rs::finish` 的 message_delta usage 为四字段。
- **2027.2 [测试] 流式 usage 单测 + 回归** —— 新增 message_delta 含 input/cache_read 用例；跑 cargo test。

## 提交策略（scoped）

1. `docs(task-plan)`: prd/feature/research/progress（本任务）。
2. `fix(transform)`: streaming.rs（finish 回传完整 usage）+ 测试。

派生/直接 scoped 提交，传精确文件清单；不碰其它文件；不推送。

## Run（验证）

- `cargo test --lib transform`（src-tauri/）。
- **无法无头验证**：真实流式缓存命中需用真实后端 + 看客户端是否收到 cache_read_input_tokens。
