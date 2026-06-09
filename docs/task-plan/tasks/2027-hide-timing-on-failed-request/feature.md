# 2027 失败请求隐藏首字/用时

## 目标

`RequestMonitor` 失败行隐藏用时/首字。

## 现状（根因）

`RequestRow` 用时/首字列仅判 `!= null`；失败请求（含拿到响应头的 5xx）仍带 durationMs/firstByteMs，会显示误导性计时。悬停 `TokenDetail` 同理。

## 关键文件/落点

- `src/components/business/RequestMonitor.tsx`：`RequestRow` 用时/首字单元格 + `TokenDetail` 首字/耗时行，按 `log.isError` 隐藏。
- `src/__tests__/RequestMonitor.test.tsx`：新增失败行断言。

## 任务拆解

- 2027b.1 RequestRow/TokenDetail 按 isError 隐藏计时 + 单测。

## 验收标准

见 prd.md。

## 测试点

- 失败行（isError=true）用时/首字渲染为 `—`，成功行不受影响。

## 提交策略

- `fix(ui)`: RequestMonitor + 测试，单提交。docs/progress 另提交。
