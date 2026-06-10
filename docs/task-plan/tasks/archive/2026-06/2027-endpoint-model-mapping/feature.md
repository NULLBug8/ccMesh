# 2027 端点模型映射（入站→出站）

## 目标

端点级 入站→出站 模型映射：UI 配置弹窗 + 对外展示并入 + 运行时路由匹配与请求体改写。

## 现状（根因）

见 `research/codebase.md`。要点：可用模型 = `model?[model]:models`，三处消费（卡片/ModelList//v1/models）；forward 重试循环内按 ep.model 改写；任务5 `filter_by_model` 仅认 `e.models`。映射需新增字段并贯穿展示、路由匹配、forward 改写。

## 关键文件/落点

后端：
- `src-tauri/src/models/endpoint.rs`：新增 `ModelMapping{from,to}`、`Endpoint.model_mappings: Vec<ModelMapping>`、Create/Update 请求加 `model_mappings`（Update 为 Option）。
- `src-tauri/src/modules/storage/migration.rs`：v7 `ALTER TABLE endpoints ADD COLUMN model_mappings TEXT NOT NULL DEFAULT '[]';` + 列存在性单测。
- `src-tauri/src/modules/storage/endpoint_repo.rs`：COLS 加列；row_to_endpoint 反序列化；create/update INSERT/UPDATE 序列化；往返/clone 单测。
- `src-tauri/src/commands/endpoint.rs`：`clone_endpoint` 复制 `model_mappings`。
- `src-tauri/src/modules/proxy/resolver.rs`：新增 `advertised_models(ep)->Vec<String>`、`resolve_outbound(ep,inbound)->Option<String>`；`filter_by_model` 改为按 `advertised_models` 匹配。单测。
- `src-tauri/src/modules/proxy/forward.rs`：构造 attempt_body 处用 `resolve_outbound` 得出有效出站模型，替换现 ep.model 逻辑（transform 与直通两分支统一）。
- `src-tauri/src/modules/proxy/server.rs`：`models_route` 用 `advertised_models` 聚合。

前端：
- `src/services/modules/endpoint.ts`：`ModelMapping{from,to}`；`Endpoint.modelMappings`；Create/Update 加可选 `modelMappings`。
- `src/pages/Endpoints/_components/ModelMappingDialog.tsx`（新）：映射编辑弹窗。
- `src/pages/Endpoints/_components/EndpointCard.tsx`：actions 加映射图标按钮（测试与克隆之间）；可用性 hover 用并入展示集；test popup 维持出站模型。
- `src/pages/Endpoints/_components/ModelList.tsx`：分组模型用并入展示集（含映射入站名）。
- 可选：`src/lib`（或就近）前端派生展示集工具，避免与卡片/ModelList 重复。

## 任务拆解

- 2027d.1 后端数据层：模型字段 + 迁移v7 + repo 往返 + clone（单测）。
- 2027d.2 后端选择/改写：resolver `advertised_models`/`resolve_outbound` + `filter_by_model` 接入 + forward 改写 + models_route（单测）。
- 2027d.3 前端类型 + 映射弹窗 + 卡片图标入口。
- 2027d.4 前端展示并入：卡片 hover + ModelList 显示映射入站名。

## 数据契约

```rust
#[serde(rename_all="camelCase")]
pub struct ModelMapping { pub from: String, pub to: String }
// Endpoint += pub model_mappings: Vec<ModelMapping>
// CreateEndpointRequest += #[serde(default)] model_mappings: Vec<ModelMapping>
// UpdateEndpointRequest += model_mappings: Option<Vec<ModelMapping>>
```
```ts
export interface ModelMapping { from: string; to: string }
// Endpoint += modelMappings: ModelMapping[]
// CreateEndpointRequest += modelMappings?: ModelMapping[]
```
```
advertised_models(ep) = dedup_ci( (ep.model 非空?[ep.model]:ep.models) ∪ ep.model_mappings.map(.from) )
resolve_outbound(ep,inbound) = ep.model_mappings.find(.from ~=ci inbound).to
                               || (ep.model 非空 ? ep.model : inbound)
```

## 验收标准

见 prd.md Acceptance Criteria。

## 测试点

- migration v7 列存在。
- repo：model_mappings 往返；clone 复制映射。
- resolver：advertised_models 去重/含映射；filter_by_model 认入站名；resolve_outbound 命中/锁定/透传/大小写。
- 前端：tsc + vitest 不回归（弹窗交互无头不验，本地核对）。

## 提交策略

1. `docs`: prd/feature/progress + research。
2. `feat(endpoint)`: 后端数据层（model/migration/repo/clone）。
3. `feat(proxy)`: resolver + forward + models_route。
4. `feat(ui)`: 类型 + 映射弹窗 + 卡片图标。
5. `feat(ui)`: 展示并入（卡片 hover + ModelList）。
