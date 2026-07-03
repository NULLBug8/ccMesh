## 流式协议转换：完整 usage 在 finish 回传

> 一句话结论：message_start 时 input 为 0 正常，finish 必须带齐四字段 usage

**你会遇到这个问题的场景**
Anthropic SSE 流式网关从 OpenAI 流式 chunk 聚合 usage（input/output/cache_creation/cache_read），在 `message_start` 与 `message_delta`（finish）事件里回传给客户端。

**为什么会出错**
OpenAI 首个 chunk 常不含完整 usage，`message_start` 时 input/cache 为 0 是预期。若在 `finish()` 的 `message_delta` 只填 `output_tokens` 而忽略已累积的 input/cache，客户端与计费展示会长期显示 input=0，而网关内部统计却正确——bug 仅在 finish 路径。

**正确做法**
- 流式上下文维护 usage 四字段，随 chunk 更新
- `message_start`：可只带已知部分，文档化「input 稍后补齐」
- `[DONE]` / finish 的 `message_delta`：**必须**从 ctx 回传 input、output、cache_creation、cache_read
- 单测：mock 多 chunk 流，断言 finish 事件四字段非零（当上游有 reporting 时）

**反例**
❌ 错误：`finish()` 仅 `usage: { output_tokens: ctx.output }`  
✅ 正确：finish 带齐 ctx 中已更新的 input + cache_* + output

---
_最后更新：2026-06-28_
