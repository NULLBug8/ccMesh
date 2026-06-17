# 2032 启动行为 + 点亮收尾 + 卡片URL放开

## 目标
落地启动行为三开关（自启动/静默/自动运行）、点亮模型对映射与下拉的收尾、亮色 CSS 可读性、卡片 URL 放开。

## 现状（根因）
- 无开机自启能力（Cargo/前端均无 autostart 插件）。
- 窗口 visible:false，前端 reveal 无条件显示，无静默通道；代理不自动启动（仅手动/托盘）。
- ModelMappingDialog 出站用 outboundModels（全量 models），未按 activeModels 过滤。
- 亮色 :root 的 --ink-mute(#71717a)/--ink-disabled(#a1a1aa) 偏淡；EndpointForm 未点亮标签带 opacity-60。
- capabilities opener:allow-open-url 仅放行 github，端点 URL 被拦。

## 关键文件/落点
- 后端配置：`src-tauri/src/models/config.rs`（AppConfig 加 silent_start/auto_run + Default）；
  `src-tauri/src/modules/storage/config_repo.rs`（get_config 解析 silentStart/autoRun；SAFE_CONFIG_KEYS 加键）。
- 自启动插件：`src-tauri/Cargo.toml`（tauri-plugin-autostart）；`src-tauri/src/lib.rs`（注册 plugin）；
  `src-tauri/capabilities/default.json`（autostart 权限）；`package.json`（@tauri-apps/plugin-autostart）。
- 自动运行：`src-tauri/src/lib.rs` setup 末尾按 auto_run 起代理。
- 静默启动：`src/lib/boot.ts`（reveal 增「静默跳过」）；`src/main.tsx`（3s 兜底前判静默）；
  `src/hooks/useThemeSync.ts`（首屏 reveal 判静默）。
- 设置 UI：`src/pages/Settings/index.tsx`（新增「启动行为」section）；`src/services/modules/config.ts`（AppConfig 加 silentStart/autoRun）。
- 模型映射：`src/services/modules/endpoint.ts`（新增 litOutboundModels）；`src/pages/Endpoints/_components/ModelMappingDialog.tsx`（改用之）。
- CSS：`src/index.css`（:root ink-mute/ink-disabled 加深）；`src/pages/Endpoints/_components/EndpointForm.tsx`（未点亮标签去 opacity-60）。
- 卡片 URL：`src-tauri/capabilities/default.json`（opener 放开 http/https）。
- 测试：`src-tauri/src/modules/storage/config_repo.rs`（#[cfg(test)]）；`src/__tests__/endpoint.test.ts`（新增）。

## 任务拆解
- 2032.1 后端配置扩展：AppConfig + config_repo 解析 + SAFE_CONFIG_KEYS + 单测（后端）
- 2032.2 自启动插件接入：Cargo + lib.rs 注册 + capability + 前端依赖（全栈）
- 2032.3 自动运行：lib.rs setup 按 auto_run 启动代理 + emit 状态（后端）
- 2032.4 静默启动：boot/main/useThemeSync 按 silentStart 跳过 reveal（前端）
- 2032.5 设置页「启动行为」类别 + config 类型扩展（前端）
- 2032.6 模型映射按点亮：endpoint.ts litOutboundModels + ModelMappingDialog + 单测（前端）
- 2032.7 CSS 亮色文字加深 + 未点亮标签去灰（前端）
- 2032.8 卡片 URL 放开：opener capability http/https（配置）
- 2032.9 校验收尾：gateway/v1 + 新建配置端口/点亮下拉验证 + 整体回归（全栈）

## 数据契约
```rust
// AppConfig 新增
pub silent_start: bool, // 默认 false
pub auto_run: bool,     // 默认 true
```
```ts
// AppConfig(前端) 新增
silentStart: boolean;
autoRun: boolean;
// endpoint.ts
export function litOutboundModels(ep: Pick<Endpoint,"model"|"models"|"activeModels">): string[];
// 锁定 model→[model]；activeModels 非空→activeModels∩models 顺序；空→models
```
config 键：`silentStart` / `autoRun`（字符串 "true"/"false"，沿用 parse_bool）。

## 验收标准
见 prd.md Acceptance Criteria。

## 测试点
- config_repo：silentStart 默认 false / autoRun 默认 true；写 "true"/"false" 往返正确。
- litOutboundModels：锁定 model；activeModels 子集过滤与顺序；activeModels 空回退全部。

## 提交策略（按模块，scoped，docs 先行）
1. docs：本任务 task.md/prd.md/feature.md/context.jsonl/research + progress.csv
2. 后端配置+自动运行：models/config.rs、config_repo.rs、lib.rs（autorun 部分）
3. 自启动插件：Cargo.toml、lib.rs（plugin 注册）、capabilities/default.json（autostart+opener）、package.json/pnpm-lock
4. 前端纯逻辑+单测：services/modules/endpoint.ts、__tests__/endpoint.test.ts
5. 前端集成/UI：boot.ts、main.tsx、useThemeSync.ts、Settings/index.tsx、services/modules/config.ts、ModelMappingDialog.tsx、index.css、EndpointForm.tsx
（capabilities 同时含 opener 放开，可与自启动一提交）
