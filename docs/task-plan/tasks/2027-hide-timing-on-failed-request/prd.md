# 失败请求隐藏首字/用时避免歧义

## Goal

请求失败（`isError`）时，实时请求监控的「用时」「首字」展示为 `—`（表格列与悬停卡均不显示数值），避免失败请求的计时数据造成误读。

## Requirements

- 表格「用时」「首字」列：`log.isError` 为真时显示 `—`。
- 悬停 Token 卡：失败时不渲染「首字」「耗时」两行。

## Acceptance Criteria

- [ ] 状态 200 的成功行照常显示用时/首字。
- [ ] 失败行（4xx/5xx/ERR，`isError=true`）用时/首字列均为 `—`，悬停卡无首字/耗时行。

## Definition of Done

- `npx tsc --noEmit` 与 `npx vitest run` 通过；新增失败行单测。

## User Stories

- 作为用户，我希望失败请求不展示首字/用时，以免把失败耗时误当作真实延迟。

## Implementation Decisions

- 以 `log.isError` 作为失败判定（后端对 4xx/5xx 及全失败兜底均置 is_error=true）；3xx 极少见不特殊处理。

## Out of Scope

- 后端记录逻辑不变（仍照常入库，仅前端展示按失败态隐藏）。
