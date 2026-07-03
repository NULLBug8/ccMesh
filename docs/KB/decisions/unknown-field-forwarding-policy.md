## Claude→OpenAI 未知字段转发决策框架

> 一句话结论：默认不转发 body 未知字段；显式缓存/会话字段单独决策

**你会遇到这个问题的场景**
Anthropic Messages 请求含 `metadata`、`cache_control` 等扩展字段，转换到 OpenAI Chat 时是否保留？部分严格 OpenAI 兼容后端拒未知 top-level 字段；部分中转又依赖 body 内字段做路由或缓存。

**为什么会出错**
「全量透传 body」在严格后端触发 400；「全剥离」又丢失显式 prompt cache（Anthropic `cache_control`）带来的成本优势。metadata 与会话标识混在 body 和 header 两套通道，容易重复或遗漏。

**正确做法**
- 默认：转换器只输出目标 schema 已知字段，未知字段不转发
- `cache_control`：若业务依赖 prompt cache，在 system/user content 合并路径保留，并定义冲突块丢弃规则（单测守护）
- `metadata`：转换层默认剥离；若需会话关联，**推荐** HTTP 头透传（如 `x-session-id`），而非塞进 OpenAI body——未实现时勿在 body 保留
- 特殊中转需求：可用显式配置开关开启 body 字段白名单，默认关（多数项目 YAGNI 暂不建）
- 每个字段写清「保留 / 剥离 / 改道 header」及单测

**反例**
❌ 错误：Anthropic metadata 原样 merge 进 OpenAI JSON body  
✅ 正确：剥离 metadata；需要时用 `x-session-id` 等头透传

---
_最后更新：2026-06-28_
