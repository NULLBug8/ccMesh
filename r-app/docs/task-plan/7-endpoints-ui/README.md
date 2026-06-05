# 阶段 7：端点管理前端

> 任务总线：[../../TASKS.md](../../TASKS.md) ｜ 规划索引：[../README.md](../README.md)
> 起点参考：旧版 ccNexus 源码 `docs/origin/ccNexus/`（Go + Wails，仅参考不复用）

## 阶段目标

实现端点管理前端：列表与卡片、表单（CodeMirror JSON 编辑）、多维筛选、克隆与测试交互、拖拽排序（筛选时禁用）。全部复用设计基线（Dark Stripe token + shadcn + StatusDot/软 Badge）。交付 **里程碑 M5 完整前端** 的端点部分。

## 前置依赖

- 阶段 0（P0-8 布局/页面骨架）；
- 阶段 4（P4-3 CRUD/排序、P4-4 克隆、P4-5 测试、P4-7 健康脱敏命令）。

## 任务清单

### P7-1 端点列表与卡片
- 所属层：React
- 文件：`src/pages/Endpoints/index.tsx`、`src/pages/Endpoints/_components/EndpointCard.tsx`、`src/hooks/useEndpoints.ts`、`src/services/modules/endpoint.ts`(endpointApi)
- 实现要点：TanStack Query 拉取端点列表；EndpointCard 展示名称、URL（脱敏 Key）、类型 Badge、启用 Switch、测试状态 TestBadge；编辑/删除/克隆/测试操作入口。
- 前置：P4-3, P4-4, P4-5, P4-7, P0-8
- 验收：列表正确渲染，启用切换与删除即时生效。
- PRD Story：1, 53, 55

### P7-2 端点表单（CodeMirror JSON 编辑）
- 所属层：React
- 文件：`src/pages/Endpoints/_components/EndpointForm.tsx`
- 实现要点：用 shadcn Dialog + Input/Label/Select 编辑基础字段；高级/原始配置用 CodeMirror（`@uiw/react-codemirror` + `@codemirror/lang-json`）做 JSON 编辑与校验；transformer 用 Select 选择（claude/openai）；提交前前端校验 + 后端校验。
- 前置：P7-1
- 验收：创建/编辑端点成功，JSON 非法时阻止提交并提示。
- PRD Story：1, 11, 12

### P7-3 端点筛选
- 所属层：React
- 文件：`src/pages/Endpoints/_components/FilterBar.tsx`、`src/stores/modules/filters.ts`
- 实现要点：按类型（claude/openai 等）、可用性（available/unknown/unavailable）、启用状态多选筛选；筛选状态存 Zustand 并持久化（localStorage）；展示是否有激活筛选。参考旧版 `filters.js`。
- 前置：P7-1
- 验收：多维筛选正确叠加，刷新后保持筛选状态。
- PRD Story：62, 63, 64

### P7-4 克隆与测试交互
- 所属层：React
- 文件：`src/pages/Endpoints/_components/CloneButton.tsx`、`TestBadge.tsx`
- 实现要点：克隆按钮调用 `clone_endpoint` 并刷新列表 + toast；测试按钮调用 `test_endpoint`，期间显示 loading，结果用 TestBadge（成功/失败/未测试三态，lucide 图标 + 颜色）。
- 前置：P7-1, P4-4, P4-5
- 验收：一键克隆生成副本；测试结果实时反映并持久化展示。
- PRD Story：65, 66, 67, 68

### P7-5 拖拽排序（筛选时禁用）
- 所属层：React
- 文件：`src/pages/Endpoints/_components/DnDList.tsx`
- 实现要点：实现拖拽排序（可用原生 HTML5 DnD 或轻量库，结合 Motion 过渡），拖拽结束调用 `reorder_endpoints` 持久化；当存在激活筛选时禁用拖拽（避免顺序错乱），并给出提示。参考旧版 story 70。
- 前置：P7-3, P4-3
- 验收：拖拽后顺序持久化；筛选激活时拖拽被禁用。
- PRD Story：69, 70

## 参考材料（origin）

> 路径相对 `docs/origin/ccNexus/`

| 关注点 | 文件 | 说明 |
|--------|------|------|
| 端点列表/卡片/操作 UX | `cmd/desktop/frontend/src/modules/endpoints.js` | 编辑/删除/克隆/测试交互 |
| 多维筛选 | `cmd/desktop/frontend/src/modules/filters.js` | 类型/可用性/启用筛选叠加 + 持久化 |
| 表单弹窗 | `cmd/desktop/frontend/src/modules/modal.js` | 端点表单字段与校验 |
| 端点组件（另一实现） | `cmd/server/webui/ui/js/components/endpoints.js` | 列表/筛选另一参考 |
| 后端契约 | `internal/service/endpoint.go` | 字段、克隆命名、测试状态 |
| 筛选样式参考 | `cmd/desktop/frontend/src/styles/filters.css` | 仅交互参考，样式用 Dark Stripe |

> 前端实施约束（见 TASKS.md §二 设计系统基线）：一律用 token、按钮 pill、状态用 `StatusDot` + 软 `Badge`、数值用 `TabularText`。

## 完成判据（里程碑 M5 之一）

- 端点列表/卡片渲染、启用切换、删除即时生效；
- 表单 JSON 校验拦截非法提交；多维筛选叠加并持久化；
- 克隆生成不冲突副本、测试三态展示；拖拽排序持久化且筛选时禁用。
