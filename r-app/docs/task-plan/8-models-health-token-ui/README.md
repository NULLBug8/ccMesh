# 阶段 8：模型列表 / 健康检查 / Token 计数前端

> 任务总线：[../../TASKS.md](../../TASKS.md) ｜ 规划索引：[../README.md](../README.md)
> 起点参考：旧版 ccNexus 源码 `docs/origin/ccNexus/`（Go + Wails，仅参考不复用）

## 阶段目标

实现模型列表前端（含缓存/刷新提示）、Dashboard 健康概览（状态/启用数/脱敏端点）、Token 计数工具。交付 **里程碑 M5 完整前端** 的辅助能力。

## 前置依赖

- 阶段 4（P4-6 模型缓存、P4-7 健康脱敏、P4-8 Token 计数命令）；
- 阶段 1（P1-7 代理控制，Dashboard 已存在）。

## 任务清单

### P8-1 模型列表前端
- 所属层：React
- 文件：`src/pages/Endpoints/_components/ModelList.tsx` 或 `src/pages/Endpoints/index.tsx` 内嵌、`src/hooks/useModels.ts`
- 实现要点：展示可用模型列表，提供刷新按钮（`get_models(force_refresh=true)`）；命中缓存与刷新状态用 Badge/时间戳提示。
- 前置：P4-6, P0-8
- 验收：列表展示模型，刷新拉取最新，缓存期内不重复请求。
- PRD Story：49, 50, 51, 52

### P8-2 健康概览前端
- 所属层：React
- 文件：`src/pages/Dashboard/index.tsx`、`src/pages/Dashboard/_components/HealthOverview.tsx`、`src/components/business/StatCard.tsx`
- 实现要点：Dashboard 展示健康状态、已启用端点数量、脱敏端点列表（Card 网格）。
- 前置：P4-7, P1-7
- 验收：显示正确启用数量与脱敏 Key。
- PRD Story：53, 54, 55

### P8-3 Token 计数前端（可选工具）
- 所属层：React
- 文件：`src/pages/Settings/_components/TokenCounter.tsx`（设置页工具区）
- 实现要点：提供文本输入估算 Token 数（调用 `count_tokens`），展示 input/system 估算结果。
- 前置：P4-8
- 验收：输入文本返回合理 Token 估算。
- PRD Story：56, 57, 58

## 参考材料（origin）

> 路径相对 `docs/origin/ccNexus/`

| 关注点 | 文件 | 说明 |
|--------|------|------|
| 统计/仪表盘前端 | `cmd/desktop/frontend/src/modules/stats.js` | 概览/卡片布局 |
| Dashboard / Stats 组件 | `cmd/server/webui/ui/js/components/{dashboard,stats}.js` | 健康概览另一参考 |
| 模型 API 文档 | `docs/models_api.md` | 模型列表接口/字段 |
| Token 估算后端 | `internal/tokencount/estimator.go` | 估算口径与前端展示对齐 |
| API Key 脱敏 | `internal/proxy/utils.go` | `maskAPIKey`（前端只展示脱敏值） |

## 完成判据（里程碑 M5 之一）

- 模型列表展示 + 刷新，缓存期内不重复请求；
- Dashboard 正确显示健康状态、启用端点数与脱敏 Key；
- Token 计数工具返回合理估算。
