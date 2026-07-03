## 错误响应体限长采集链路

> 一句话结论：非 2xx 读 body，限 4096 字节入库，仅错误行展示

**你会遇到这个问题的场景**
API 网关记录每次 forward 的请求/响应元数据，运维需在 UI 请求日志里看到「为什么 403/502」，而不是只看 status code。

**为什么会出错**
若 forward 非 2xx 直接 relay 不读 body，DB 无 error 详情。若无限长存 body，SQLite 膨胀且 UI 卡顿。成功请求存 error body 则浪费且误导。

**正确做法**
- forward 非 2xx 路径：`bytes = resp.bytes()` 缓冲（与 rectifier 共用同一读点）
- 400/401 等不重试错误：缓冲后立即写入 `error_body`；403/5xx 重试路径先存 `last_error_body`，轮换耗尽后再入库
- 写入 `RequestRecord.error_body`：仅 `is_error=true` 时存
- 上限 **4096 字节**，超出追加截断标记（如 `... [已截断，原长度 N 字节]`）
- 前端 HoverCard（悬停详情卡片）展示；老数据 NULL 时降级「无错误详情」
- 与 rectifier 决策共用缓冲 bytes，避免二次读流

**反例**
❌ 错误：所有响应全量 body 入库  
✅ 正确：仅 error + 4KB cap + 截断后缀

---
_最后更新：2026-06-28_
