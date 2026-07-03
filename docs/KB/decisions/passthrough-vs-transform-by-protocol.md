## 透传/转换判定对齐「上游原生协议」

> 一句话结论：上游按端点协议判定透传/转换；入站识别勿硬编码 UA

**你会遇到这个问题的场景**
网关同时服务多种客户端（Chat CLI、Responses CLI、Anthropic SDK）和多种上游（OpenAI 兼容、Anthropic 原生、Responses 原生）。需决定某条请求是原样转发还是经转换器改写。

**为什么会出错**
若用 URL 路径（含 `/responses`）或 User-Agent 判定，客户端升级或自定义 UA 会导致误判。本质是：**上游是否原生支持客户端发出的协议**，与客户端叫什么无关。

**正确做法**
- 端点配置显式标记 `transformer` / `protocol`（如 openai-chat vs openai-responses）
- **上游侧**判定：客户端协议 == 端点原生协议 → 透传；否则 → 转换
- **入站侧**识别客户端协议：优先显式协议头/配置；若暂用 URL path（如 `/chat/completions` vs `/responses`），文档化为例外并计划收敛
- User-Agent 不参与透传/转换判定（仅影响出站头伪装）
- 参考 moon-bridge 等成熟实现：`Protocol==openai-response` 透传，否则转 Chat
- 鉴权、模型列表探测与 forward 共用同一 protocol 枚举，避免三处各写一套

**反例**
❌ 错误：用 UA 决定 passthrough → ✅ 正确：UA 只写出站头，判定看端点 `transformer`  
❌ 错误：`if path.contains("responses") { passthrough }`（与上游协议混用）  
✅ 正确：`if endpoint.native_protocol == client.protocol { passthrough } else { transform }`

---
_最后更新：2026-06-28_
