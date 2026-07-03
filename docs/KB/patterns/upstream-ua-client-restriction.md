## API 网关 UA：按端点类型伪装 CLI 而非透传客户端

> 一句话结论：按协议类型用全局 CLI UA；空串才透传客户端

**你会遇到这个问题的场景**
桌面壳（Electron/Tauri）内的 HTTP 客户端转发 LLM 请求到 new-api 类聚合上游。上游默认**不透传**入口 User-Agent 到真实渠道，但用 UA 做客户端亲和或 `channel:client_restricted` 限制。

**为什么会出错**
若 forward 透传 WebView/Electron 的 UA，上游视为「浏览器客户端」拒绝或路由到错误渠道。OpenAI/Codex 官方 CLI 有固定 UA 与 `originator` 头，聚合层按此识别合法 CLI 流量。

**正确做法**
- 按**上游协议类型**（OpenAI Chat / Codex Responses / Claude 等）从应用级配置取默认 UA（如 Codex CLI 字符串 + `originator` 头）
- 仅当用户**显式保存空字符串**时才透传原始客户端 UA
- UA 与鉴权同级属应用/类型级配置；若需 per-endpoint 覆盖，扩展端点模型后再写
- 403 + `client_restricted` 排查时对照实际发出的 UA（可在 forward 错误路径加诊断日志）

**反例**
❌ 错误：`req.headers().insert(UA, incoming_ua)` 一律透传  
✅ 正确：OpenAI 类端点缺省替换为 CLI UA，用户空串才透传

---
_最后更新：2026-06-28_
