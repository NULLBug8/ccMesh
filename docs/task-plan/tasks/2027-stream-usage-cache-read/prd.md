# 流式 message_delta 回传完整 usage（含 cache_read）

## Goal
流式 Claude→OpenAI 转换把完整 usage（input/output/cache_read/cache_creation）经最终 message_delta 回传客户端，修复客户端流式下 cache_read/input_tokens 恒 0。

## Requirements
- `finish()` 的 message_delta usage 携带四字段，取自 [DONE] 时已就绪的累积 ctx。
- 不破坏现有流式行为与统计。

## Acceptance Criteria
- [ ] 末尾 usage chunk 带 prompt_tokens_details.cached_tokens 时，message_delta usage 含正确 input_tokens 与 cache_read_input_tokens。
- [ ] 无 usage 时各字段为 0、不报错。
- [ ] 现有 streaming 测试不回归；cargo test 通过。

## Definition of Done
实现 + 单测；进度写回；scoped 提交；真实流式缓存命中需真实后端验证（显式声明）。

## User Stories
- 作为经本网关流式对接 OpenAI 兼容后端的用户，我希望客户端流式能看到缓存读取与输入 token，以便缓存/成本显示正确。

## Implementation Decisions
- 在最终 message_delta（[DONE] 时已延迟发出）回传完整 usage，对齐 cc-switch。
- message_start 维持 0、不阻塞等待 usage（与 cc-switch 一致）。
- 四字段恒发（含 0），与 message_start 结构一致。

## Testing Decisions
- 流式单测：content + 末尾 usage-only chunk（带 cached_tokens）→ 断言 message_delta 含 input/output/cache_read。

## Out of Scope
- message_start usage 改造；非流式路径；网关自身统计（均已正确或无需改）。

## Technical Notes
- 落点 streaming.rs::finish；根因/参考见 research/streaming-usage.md。
