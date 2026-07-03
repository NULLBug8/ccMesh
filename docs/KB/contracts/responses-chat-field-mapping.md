## Responses↔Chat 直转的关键字段与编码陷阱

> 一句话结论：字段名、token 限额键、arguments 双重编码是 Responses 桥接三大坑

**你会遇到这个问题的场景**
客户端走 OpenAI Responses API（`/v1/responses`），上游却是 Chat Completions 兼容端点。需在网关做请求/响应/流式 SSE 双向转换。

**为什么会出错**
Responses 与 Chat 的 schema 同名不同义：`max_output_tokens` 对应 Chat 的 `max_completion_tokens`（不是 `max_tokens`）；`reasoning.effort` 嵌套对象对应 Chat 顶层 `reasoning_effort`；Chat 侧 `function.arguments` 类型是 **JSON 字符串**，入站需去外层引号解析、出站需再包一层引号。流式还需完整 SSE 事件序列与递增 `sequence_number`。

**正确做法**
- 请求：`max_output_tokens` → `max_completion_tokens`；`reasoning.effort` → `reasoning_effort`
- Tool I/O：入站 Responses output 解析后写入 Chat messages；出站 Chat tool_calls 的 arguments 做 JSON 字符串编码
- 流式：维护 item_id 前缀约定（如 `resp_` / `msg_` / `fc_`），每事件递增 `sequence_number`
- 用外部参考实现（如 Go moon-bridge）逐字段建映射表并单测锁定
- 不支持的 reasoning 档位（如 xhigh）在转换层降级并重试，勿透传致 400

**反例**
❌ 错误：`max_output_tokens` 直接映射为 `max_tokens`  
✅ 正确：映射为 `max_completion_tokens`

---
_最后更新：2026-06-28_
