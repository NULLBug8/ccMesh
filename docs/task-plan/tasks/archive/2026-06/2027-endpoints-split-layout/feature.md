# 2027 端点管理页上下分栏

## 目标

Endpoints 页固定头部 + 上下 60/40 比例滚动区。

## 现状（根因）

`src/pages/Endpoints/index.tsx` 根为 `mx-auto flex max-w-4xl flex-col gap-5`，随内容增高，整页在 `AppLayout` 的 `main`(`overflow-y-auto`) 内单一滚动；标题/筛选/端点列表/模型列表顺序排布，无分区与高度约束。

## 关键文件/落点

- `src/pages/Endpoints/index.tsx`：根容器加 `h-full`；拆「固定头部(标题+FilterBar, shrink-0)」+「端点区 `flex-[3] min-h-0 overflow-y-auto`」+「模型区 `flex-[2] min-h-0 overflow-y-auto`」。
- 不改 `AppLayout.tsx`、`DnDList.tsx`、`ModelList.tsx`。

## 任务拆解

- 2027c.1 重排 Endpoints 根布局为固定头部 + 60/40 上下滚动区。

## 数据契约

无（纯布局）。

## 验收标准

见 prd.md。

## 测试点

- `tsc` 通过；`vitest` 全量不回归（无该页单测，确保无连带破坏）。
- 视觉滚动/比例为无头不可验，本地核对。

## 提交策略

- `style(ui)`: Endpoints 页布局，单提交；docs/progress 另提交。
