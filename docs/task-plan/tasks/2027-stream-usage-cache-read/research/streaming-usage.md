# 流式 usage 回传缺口调研

## 问题
Claude→OpenAI 流式转换路径，客户端收不到 `cache_read_input_tokens`（连 `input_tokens` 都是 0）。

## 根因（本项目）
- `streaming.rs:message_start`（:54-59）的 usage 取 ctx，但此时 OpenAI usage chunk 未到（forward 传 input=0、cache 默认 0）→ message_start 里 input/cache 全 0。
- `streaming.rs:finish`（:308-314）的 `message_delta` usage **只有 output_tokens** → 后来从 usage-only chunk 拿到的 input/cache_read **没回传客户端**。
- 注：`process_chunk` 顶部读 `chunk.usage` 不依赖 choices，usage-only chunk 会更新 ctx（input/output/cache_creation/cache_read，cache_read 经 :327 `cache_read_tokens` 兜底 prompt_tokens_details.cached_tokens 等）。`finish()` 在 `[DONE]` 才发 message_delta，**那时 ctx 已是完整值**。
- 网关自身统计不受影响：`forward.rs` 流结束用 `converter.usage()` 读 ctx，已正确入库；缺口**只在发给客户端的 SSE**。

## 参考实现 cc-switch（已验证）
`E:\myCode\cc-switch\src-tauri\src\proxy\providers\streaming.rs`：
- `build_anthropic_usage_json`（:102-113）：usage 带 input_tokens+output_tokens，cache_read/creation 存在则带。
- 最终 message_delta（延迟到 [DONE]）带**完整 usage**。
- 测试 `test_usage_only_chunk_after_finish_reason_updates_message_delta_usage`（:993）断言最终 message_delta 含 `input_tokens:13312` `cache_read_input_tokens:100`，Claude Code 接受。

## 结论
只需把 `finish()` 的 message_delta usage 从 `{output_tokens}` 改为带全 4 字段（input/output/cache_creation/cache_read，由 [DONE] 时已就绪的 ctx 提供）。极小改动，顺带修复流式 input_tokens 给客户端恒 0。
