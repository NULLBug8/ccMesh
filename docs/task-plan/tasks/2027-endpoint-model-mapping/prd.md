# 端点模型映射（入站→出站）

## Goal

为每个端点提供「模型映射」配置：把客户端请求的入站模型名映射到该端点已配置的出站（真实）模型名。客户端可用入站名请求，网关据此路由到该端点并把请求体 model 改写为出站名转发上游；对外公布的可用模型 = 原可用模型 ∪ 映射入站名。

## Requirements

- 端点卡片在「测试连通性」与「克隆」图标之间新增「映射」图标，点开映射弹窗。
- 弹窗：每行 左=入站模型(手动输入) · 中=转换图标 · 右=出站模型(下拉，仅该端点可用模型)；支持增/删多行；保存持久化到该端点。
- 端点无任何可用模型（models 空且无锁定 model）时不允许配置，提示先配置模型。
- 对外可用模型展示（/v1/models、ModelList、卡片可用性 hover）= 原可用模型 ∪ 映射入站名（去重）。
- 运行时：入站名参与端点路由匹配（filter_by_model）；命中端点后请求体 model 改写为映射出站名转发上游。

## Acceptance Criteria

- [ ] 卡片图标顺序：测试 → 映射 → 克隆 → 编辑 → 删除。
- [ ] 弹窗右侧出站下拉项 = 该端点 `model?[model]:models`；左侧入站可手输。
- [ ] 保存后 `/v1/models` 与卡片/ModelList 出现入站映射名（不重复出站名）。
- [ ] 请求入站名 X（仅存在于映射）能路由到声明该映射的端点，且上游收到的 model = 出站名 Y。
- [ ] 克隆端点连同映射一起复制。
- [ ] `cargo test` / `npx tsc --noEmit` / `npx vitest run` 通过；新增映射解析/路由单测。

## Definition of Done

- 后端：迁移 v7 + 模型字段 + repo 往返 + forward 改写 + resolver 匹配，含单测。
- 前端：映射弹窗 + 卡片图标 + 展示并入，类型同步。
- GUI 交互/真实转发为无头不可验，显式声明本地核对。

## User Stories

- 作为用户，我希望把第三方/自定义模型名映射到端点真实模型，以便客户端用熟悉的名字请求而网关自动转发到正确上游模型。

## Implementation Decisions

- 运行时语义＝**重写出站 + 入站参与路由**（用户决定）。入站映射名并入端点可匹配模型集合，保证与任务5 的 `filter_by_model` 自洽。
- 端点无可用模型时**禁止配置映射**（用户决定），出站下拉为空并提示。
- 映射存于端点：新增 `model_mappings: Vec<{from,to}>`，JSON TEXT 列（迁移 v7），随 Create/Update/Clone 流转；映射保存复用 `update_endpoint`（加 modelMappings 字段），不新增命令。
- 优先级（proposed）：解析出站 = 映射命中 → 否则锁定 model → 否则透传入站名。即**映射 > 锁定 > 透传**。
- 测试连通性弹窗（test popup）仍只用**出站(真实)模型**：test 直连上游、不经网关，入站名上游不认。卡片"可用性 hover"用并入后的展示集。
- 映射仅在独立弹窗配置，**不**并入 EndpointForm 编辑表单（贴合需求"映射入口"独立）。
- 派生公共函数避免三处重复：`advertised_models(ep)`（展示+路由匹配）与 `resolve_outbound(ep,inbound)`（forward 改写）。

## Testing Decisions

- resolver：`filter_by_model` 认入站映射名；新增 `advertised_models`/`resolve_outbound` 单测（命中/未命中/锁定/透传/去重/大小写）。
- endpoint_repo：model_mappings 往返 + clone 复制。
- migration：v7 列存在性。

## Out of Scope

- 全局（跨端点）模型别名、正则/通配映射、双向映射。
- 历史请求按映射回溯统计。

## Technical Notes

- 大小写：入站匹配沿用 `eq_ignore_ascii_case`（与 filter_by_model 一致）。
- 弹窗 UI 视觉/真实出网转发无法无头验证。
