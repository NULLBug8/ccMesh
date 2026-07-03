## 工具 arguments JSON 规范化契约

> 一句话结论：可解析则排序键；缺省/非法降级 `{}`

**你会遇到这个问题的场景**
网关把 Anthropic `tool_use` 或流式累积的 tool delta 转成 OpenAI `function.arguments`（JSON **字符串**字段）转发给兼容后端。不同上游对空串、键序、空白串容忍度不一致。

**为什么会出错**
OpenAI 规范要求 `arguments` 是 JSON 字符串，不是对象。空串 `""` 在部分上游等价于非法 JSON object；对象键序不稳定会导致 prefix cache（前缀缓存，相同前缀复用计费）失效；流式中途对 partial JSON 排序会破坏语义。

**正确做法**
- 可解析为 JSON 的值：递归对 object 键**字母序排序**，再紧凑序列化
- 缺失 arguments 或空 object：默认 `"{}"`
- 不可解析或非法 JSON 字符串：降级为 `"{}"` 并记录 warn，勿静默透传脏串
- 纯空白串：若上游拒收，按空 object 处理（与缺省同等）
- 流式路径：**理想**仅在 tool call 参数块 finish 时对完整串 canonicalize；Anthropic↔Chat 流式 delta 路径常原样累积，finish 未必再规范化
- 非流式：在 `tool_use → tool_calls` 或 Responses 入站转换点调用

**反例**
❌ 错误：流式每个 delta 片段都做 `serde_json` 排序  
✅ 正确：非流式在转换点 `canonical_json_string`；流式 finish 再规范化（若路径支持）
❌ 错误：不可解析 arguments 原样发出 → ✅ 正确：降级 `"{}"` 并 warn

---
_最后更新：2026-06-28_
