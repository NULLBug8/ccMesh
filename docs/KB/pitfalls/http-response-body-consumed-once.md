## 4xx 必须先缓冲响应体再 relay 或 rectify

> 一句话结论：HTTP 响应体只能读一次，rectifier 与 passthrough 不能共用未缓冲流

**你会遇到这个问题的场景**
代理层收到上游 400/401/422，既要可能触发错误驱动 rectifier 重试，又要把最终错误原样返回客户端。实现里常见 `resp.bytes()` 与 `relay_passthrough(resp)` 两条路径。

**为什么会出错**
HTTP 响应 body 是单向流，读取即消费。若 4xx 路径直接 relay 而不读 body，rectifier 无法匹配错误消息；若读了 body 又试图 relay 原 Response，body 已空。两条路径互斥，必须统一为「先缓冲再决策」。

**正确做法**
- 非 2xx：先 `bytes = resp.bytes().await` 缓冲全文
- 用缓冲文本做 rectifier substring 匹配；命中则改请求体重试
- 不命中或重试仍失败：用 bytes + 原 status/headers **重建** Response 回传
- 2xx 流式路径单独处理，不与 4xx 缓冲逻辑混用同一 consume 点

**反例**
❌ 错误：400 时 `relay_passthrough(resp)`，rectifier 永远看不到 body  
✅ 正确：`let body = resp.bytes().await?` → match → rebuild 或 retry

---
_最后更新：2026-06-28_
