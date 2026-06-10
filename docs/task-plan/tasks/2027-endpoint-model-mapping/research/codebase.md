# 模型映射 — 代码调研

## 需求要点（模型映射需求.txt）

- 端点卡片在「可用性检测(测试连通性)」与「复制端点(克隆)」图标之间加「映射入口」图标，点开弹窗。
- 弹窗：左=入站模型（手动输入）、中=转换图标、右=出站模型（只能从该端点"可用模型"中选）；支持多条。
- 对外展示「可用模型」= 原可用模型 + 映射的（入站）模型。
- 含义：客户端用入站模型名请求 → 网关把请求体 model 改写为出站模型转发上游。

## 现状架构

### 端点数据模型
- `src-tauri/src/models/endpoint.rs`：`Endpoint{ model: String(锁定), models: Vec<String>(清单) }`；`CreateEndpointRequest`/`UpdateEndpointRequest` 同字段。
- `src/services/modules/endpoint.ts`：TS `Endpoint`/`CreateEndpointRequest`/`UpdateEndpointRequest`（camelCase）。
- 存储 `src-tauri/src/modules/storage/endpoint_repo.rs`：`models` 以 JSON TEXT 存（`serde_json::to_string`），`row_to_endpoint` 反序列化；COLS 常量列出所有列。
- 迁移 `migration.rs`：v2 加 `models TEXT DEFAULT '[]'`；新列照此追加（当前最高 v6）。

### "可用模型" = `model ? [model] : models` 的三处消费
1. `src/pages/Endpoints/_components/EndpointCard.tsx`：`shownModels`（可用性 hover + 测试模型 Popover）。actions 顺序：testButton(ActivityIcon) → clone(CopyIcon) → edit → delete。**映射图标插在 testButton 与 clone 之间。**
2. `src/pages/Endpoints/_components/ModelList.tsx`：按端点分组展示。
3. `src-tauri/src/modules/proxy/server.rs` `models_route`(`/v1/models`)：聚合启用端点的 `model`/`models` 公布（Anthropic/OpenAI 双格式）。

### 运行时模型改写（forward.rs handle_proxy 重试循环内）
- `model` 变量取自请求体 `model` 字段（入站模型）。
- needs_transform(Claude→OpenAI)：`transform_request(cj, Some(&ep.model))` 用 ep.model。
- 否则 `!ep.model.is_empty()`：覆盖请求体 model=ep.model。
- 否则透传客户端 model。
- **映射改写点**：在此处按"入站→出站"解析有效出站模型，替换 ep.model 当前逻辑。

### 端点选择（已于任务5新增）
- `src-tauri/src/modules/proxy/resolver.rs` `filter_by_model(enabled, model)`：仅保留 `e.models` 含该模型的端点；无则回退全量。
- **关键耦合**：若公布了入站映射名 X，但 filter_by_model 不认 X（X 只在映射里、不在 models），则请求 X 会"无端点声明"→回退全量→可能路由到不支持 X 的端点。故映射的入站名必须并入 filter_by_model 的可匹配集合。

### 克隆 / 命令
- `commands/endpoint.rs clone_endpoint`：手工逐字段复制 `CreateEndpointRequest`（含 models）。新增 model_mappings 需同步复制。
- `update_endpoint` 直通 `endpoint_repo::update`。映射可复用 update（新增 modelMappings 字段），无需新命令。

## 拟定数据结构

```rust
// 每条映射：入站名 -> 出站名
pub struct ModelMapping { pub from: String, pub to: String }
// Endpoint 新增： pub model_mappings: Vec<ModelMapping>  （JSON TEXT 存，v7 迁移）
```

## 拟定派生函数（消除三处重复 + 路由一致）
- `advertised_models(ep) = dedup( (model?[model]:models) ∪ mappings.from )` → 用于 /v1/models、ModelList、卡片 hover、filter_by_model 匹配。
- `resolve_outbound(ep, inbound) = mappings.find(from==inbound).to  ||  (ep.model 非空 ? ep.model : inbound)` → forward 改写。

## 待澄清（见 prd User Decisions）
1. 运行时语义：A 重写出站+入站参与路由 / B 仅重写不改路由 / C 仅展示。（B/C 与任务5的 filter_by_model 不自洽）
2. 出站可选项为空（端点未配置任何可用模型）时是否允许配置映射。
3. 优先级：入站名命中映射，同时该名又是已配置可用模型时以谁为准（拟：映射优先）。
