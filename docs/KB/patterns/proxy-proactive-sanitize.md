## 协议代理：主动 sanitize 优于反应式修复

> 一句话结论：能在请求构建期消除的校验错误，不要等上游 400 再重试

**你会遇到这个问题的场景**
做 OpenAI / Anthropic 兼容网关，上游对 tool call 的 `arguments` 格式、空字段、键序等有严格校验。首次请求已被上游处理并可能计费，反应式重试浪费 quota 且增加延迟。

**为什么会出错**
反应式修复（读 400 错误体 → 改 body → 重发）看似通用，但第一次失败请求往往已计入用量。严格上游（如部分国产兼容层）对 `arguments:""` 直接 400，宽松上游则静默当空对象——问题在请求形态，不在运行时才发现。

**正确做法**
- 在 transform / 请求构建层完成 JSON 规范化：空串 → `"{}"`、对象键递归排序
- 流式 tool call 仅在**整块参数收尾**时 canonicalize（若该路径实现）；Anthropic↔Chat 流式 delta 常原样累积
- 反应式 rectifier（错误驱动重试）留给无法预判的错误：thinking 签名、media fallback 等
- 两层分工：主动 sanitize 覆盖已知契约；rectifier 覆盖错误消息 substring 可识别的异常

**反例**
❌ 错误：原样转发 `arguments:""`，等 400 后再改成 `{}` 重试  
✅ 正确：出站前 `canonical_json_string`，首次即通过校验

---

## 错误驱动 Rectifier：原始错误体 substring + 单次重试

> 一句话结论：用错误消息 substring 匹配 + 一次性标志位，勿解析嵌套 JSON

**你会遇到这个问题的场景**
网关收到上游 400/422，错误体是嵌套 JSON 字符串（如 OpenAI 风格 `{ "error": { "message": "..." } }`）。需要在不改客户端的前提下修正请求体并重试一次。

**为什么会出错**
结构化解析错误 JSON 脆弱：字段名、嵌套层级因上游而异。若无限重试会死循环；若匹配过宽会误改正常请求。

**正确做法**
- `should_rectify_*` 对**原始错误消息**做小写 substring 匹配（7+ 类已知场景）
- 每种 rectifier 配独立的一次性标志位（如 `sig_rectified`），命中后仅重试一次
- 修改请求体用**原地 mutate**（如删 thinking 块、signature 字段），返回 `applied` 布尔
- 不命中或 `applied=false` 时，用已缓冲的错误字节原样回传客户端
- rectifier 改的是**上游协议格式 body**（如 Anthropic messages），不是已转换后的 OpenAI 形态

**反例**
❌ 错误：`serde_json` 解析 error.message 再 switch type → 漏掉嵌套串格式  
✅ 正确：整段 error body 转小写后 `contains("thought signature")` 等

---
_最后更新：2026-06-28_
