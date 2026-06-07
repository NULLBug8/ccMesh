# 05 — WP5 前端工具与监控修复

> 关联：[TASKS.md](./TASKS.md) · [PRD-2.md](./PRD-2.md)
> 所属层：前端（React 19 + Tailwind v4 + TanStack Query v5）
> 前置：无（本轮基线，5.1/5.2 被后续工作包复用）

## 目标

补齐 Token 单位辅助展示（R4）、修复统计页 ranged 监控"一直加载"（R6）、仪表盘 Token 单位（R5）、历史记录弹窗放大（R7）。5.1 工具函数与 5.2 日期工具是后续 WP6/WP7 的复用基线。

## 关键文件/落点

- 工具：`src/lib/utils.ts`（已有 `cn`）或新增 `src/lib/format.ts`（实现时定，倾向新增 `format.ts` 收口数值/时间格式化）
- 监控：`src/components/business/RequestMonitor.tsx`（`computeRange`/queryKey）
- 仪表盘：`src/pages/Dashboard/index.tsx`（`StatCard label="Token（今日）"`）
- 历史弹窗：`src/pages/Statistics/_components/HistoryDialog.tsx`（`DialogContent`）
- 卡片复用：`src/components/business/StatCard.tsx`（已有 `hint?: ReactNode` 槽位）
- 测试：`src/__tests__/`（已有 `RequestMonitor.test.tsx`、`Pagination.test.tsx`、`filters.test.ts`）

## 任务拆解

- **5.1** `formatTokenCompact(n: number): string`：
  - `n >= 1e8` → `≈{(n/1e8).toFixed(2)}亿`；`n >= 1e4` → `≈{(n/1e4).toFixed(2)}万`；否则 `n.toLocaleString()`（不加单位、不加 ≈）。
  - 防御：非有限值/NaN → `"0"`；负数取简单稳健策略（绝对值折算后保留负号，或直接 `toLocaleString()`，实现时取一种并测）。
  - 主数值仍展示精确值，本函数产出"辅助小字"文案。
- **5.2** 稳定时间段计算纯函数（如 `rangeFromKey(key, nowDayStartMs)` 或内部按天对齐）：
  - `today`：当日 0 点 → 次日 0 点（或当日 23:59:59.999）固定上界；`7d/30d`：以"今天 0 点"为锚按天对齐回溯 N 天；`all`：无界。
  - 关键不变量：同一 `key` + 同一"当天"输入返回**相等**的 `{startMs,endMs}`（不含每帧变化的 `Date.now()`）。
- **5.3** `RequestMonitor` 接入 5.2：用 `useMemo`（依赖 `rangeKey` 与"当天"标识）计算 range，使 queryKey 在同一筛选下稳定；移除直接把 `Date.now()` 放进 key 的写法。修复后 ranged 模式正常出数据、不再无限重取。
- **5.4** 仪表盘"Token（今日）"：`<StatCard label="Token（今日）" value={tokens.toLocaleString()} hint={<span className="text-xs text-ink-secondary">{formatTokenCompact(tokens)}</span>} />`（样式实现时定）。
- **5.5** `HistoryDialog`：`DialogContent` 由 `max-w-3xl` 放大（`max-w-5xl`/`max-w-6xl`），表格区包一层 `max-h-[...] overflow-y-auto`，标题与分页器常驻。

## 验收标准

- `formatTokenCompact`：`9999→"9,999"`、`10000→"≈1.00万"`、`125000000→"≈1.25亿"`、`9000000→"≈900.00万"`、`10000000→"≈1000.00万"`；0/非法值不抛错。
- 统计页"端点请求记录"（ranged）切换今日/7天/30天/全部均能出数据，不再持续"加载中"，同一筛选下不反复闪烁/重取。
- 仪表盘"Token（今日）"在大数值下显示辅助单位小字。
- 历史记录弹窗明显更宽，长列表可滚动、分页器可用。

## 测试点（vitest + testing-library）

- `formatTokenCompact`（纯函数）：量级边界、就近取档、两位小数、0/负数/非有限值防御。
- 稳定时间段纯函数：同 `key`/同"当天"多次调用结果相等（防 queryKey 漂移回归）；各档边界值。
- `RequestMonitor.test.tsx`（扩展）：ranged 模式在固定 mock 时间下，重复渲染不产生新的查询 key（或不重复触发 `getRequestLogs`）。
