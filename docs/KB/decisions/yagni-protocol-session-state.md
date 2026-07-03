## 勿为不适用的协议路径引入会话状态

> 一句话结论：Chat Completions 全量历史路径不需要 Responses 专属的 ID 缓存

**你会遇到这个问题的场景**
参考项目在 Responses API（OpenAI `/v1/responses`，用 `previous_response_id` 做服务端状态）与 Chat Completions 之间做桥接，维护了跨请求的 function_call 历史缓存。你在 Claude↔Chat 网关里考虑复刻同样的 Map<call_id, tool_id>。

**为什么会出错**
Responses 客户端常只发 `previous_response_id + function_call_output`，不发完整 messages，故桥接层必须重建丢失的 tool call。Chat Completions 客户端每轮发送完整 history，tool call id 在会话内自洽，无需服务端重映射。引入缓存增加复杂度却无读写点。

**正确做法**
- 先画清入站/出站协议：是否每轮带完整 messages？
- 仅当上游依赖服务端状态 ID（如 `previous_response_id`）且转换会丢 history 时才建 store
- 文档化「参考项目有、本项目不做」及 YAGNI 理由
- Session 级 metadata 提取等同理：无消费者则不建

**反例**
❌ 错误：Claude Messages↔OpenAI Chat 网关照搬 CodexChatHistoryStore  
✅ 正确：ID 原样透传，转换层无跨轮 ID 重映射需求

---
_最后更新：2026-06-28_
