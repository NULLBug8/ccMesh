## advertised_models 与路由 filter 语义一致

> 一句话结论：入站模型名必须出现在 advertised 集合，否则 filter 会漏端点或误匹配

**你会遇到这个问题的场景**
多上游网关按请求体 `model` 字段筛选可用端点（`filter_by_model`）。端点配置了模型映射（入站名 → 出站名）和 `active_models` / `models` 列表，但路由仍把请求发到未声明该模型的渠道。

**为什么会出错**
filter 依赖 **advertised_models**（对外声明可处理的入站模型名集合）。若映射后的入站名未并入 advertised，filter 认为「无端点支持该模型」而回退全量候选或错误端点。`active_models` 为空时回退 `models` 全量，可能让未点亮模型也参与匹配。

**正确做法**
- 模型映射的**入站侧名称**写入 advertised 集合（与 resolver 一致）
- 区分「展示用 models」与「路由用 advertised」；变更映射时同步更新
- `active_models` 空时的回退策略写清并单测
- forward 异常或 warn 路径打印各端点 `advertised` 列表，便于诊断误路由

**反例**
❌ 错误：只存 outbound 映射，advertised 仍为旧列表 → 请求 `gpt-4o-alias` 匹配失败  
✅ 正确：映射保存时 `advertised_models.insert(inbound_name)`

---
_最后更新：2026-06-28_
