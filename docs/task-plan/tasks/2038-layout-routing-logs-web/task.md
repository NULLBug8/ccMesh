---
id: 2038
slug: layout-routing-logs-web
title: 页面自由布局、路由规则配置、日志四段详情与 Web 配置台
status: in_progress
priority: high
layer: fullstack
owner: codex
created_at: 2026-06-24
---

# 2038 页面自由布局、路由规则配置、日志四段详情与 Web 配置台

## 背景

当前项目的桌面端 UI 已具备多菜单页、端点管理、日志与统计能力，但仍有四个明显缺口：

1. 每个菜单页缺少统一的“自由调整页面布局”能力。
2. 路由、熔断、降级规则仍散落在现有实现中，缺少独立的用户配置页。
3. 日志页无法查看单次请求的完整四段链路详情。
4. 前端目前直接依赖 Tauri IPC，尚未提供浏览器可访问的 Web 配置台。

## 本任务目标

以“桌面端能力完善优先，Web 配置台第二阶段接入”为原则，先完成需求设计、任务拆分和实施计划，再分模块推进实现与验证。

## 当前阶段

`in_progress`

## 2026-06-27 追加完成项

- 新增中转站余额查询配置：端点持久化 `balanceQuery`，支持 method/path/headers/body/JSON Path 提取。
- 新增余额查询菜单页，并在端点卡片加入单站点余额查询入口、端点表单加入余额模板配置 Tab。
- 规则配置增加模型映射策略、最大重试预算、请求超时、失败状态码、流式降级和降级温度等配置项；每项均显示示例说明。
- 模型映射路由策略默认 `site-first`，并支持 `global-native-first`；后端候选排序已接入运行时。
- `3001` 调试实例已使用旧数据目录重启，新增本地管理命令已验证注册。

## 2026-06-27 Balance template assistant
- Endpoint balance tab now supports smart balance template detection: built-in template probing, automatic apply on match, explicit all-URL-failed state with custom path re-probe, and AI generation only after a sanitized response sample is available.

## 2026-06-27 Balance AI model scope and templates
- AI-assisted balance template generation now uses a model selected from the current endpoint instead of selecting a separate endpoint.
- The backend validates that the selected model belongs to the current endpoint and rejects AI recognition when the endpoint has no models.
- Built-in balance templates were expanded and renamed around relay types or site-specific names, including `newapi`, `one-api`, `sub2api`, `voapi`, `newapi-token`, and `one-hub`.
