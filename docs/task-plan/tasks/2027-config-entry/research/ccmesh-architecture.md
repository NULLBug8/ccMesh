# ccMesh 架构调研（为「配置文件管理」页面落点）

> 来源：只读探索 `F:\IT\ccMesh`（Tauri2 + React19 + TS + Rust）。

## 1. 前端路由 / 导航（无 react-router）
- `src/main.tsx`：ThemeProvider → QueryClientProvider → TooltipProvider → App + Toaster。
- `src/App.tsx`：挂全局 hooks + `<AppLayout/>`。
- 页面切换：`src/layouts/AppLayout.tsx` 用 `lazy()` + `PAGES: Record<ViewId, ComponentType>`，由 `useLayoutStore(s=>s.activeView)` 决定渲染；`<main className="flex-1 overflow-y-auto p-8">` 包 `<Suspense>`。
- `ViewId` 定义：`src/stores/modules/layout.ts:7-13`（dashboard/endpoints/statistics/sync/logs/settings）。
- 导航：`src/layouts/navConfig.tsx` `NAV_ITEMS`（id/label/labelEn/icon）→ `SideNav.tsx` / `TopNav.tsx` → `NavItem.tsx` 点击 `setActiveView(id)`。
- **新增页面需改 4 处**：layout.ts(ViewId) + navConfig.tsx(导航项) + AppLayout.tsx(lazy+PAGES) + 新建 `src/pages/<Name>/index.tsx`（named export）。

## 2. Endpoints 页面（最相近范本，但布局不同）
- 文件：`pages/Endpoints/index.tsx` + `_components/{FilterBar,DnDList,EndpointCard,EndpointForm,JsonEditor,ModelList}.tsx`；hook `hooks/useEndpoints.ts`、`useEndpointHealth.ts`；API `services/modules/endpoint.ts`。
- 现状是**单栏上下分区 + Dialog 表单**，不是需求要的「左列表/中表单/右整合编辑器」三栏 → 三栏需自行拼装。
- 可复用模式：react-query list（`useQuery(["endpoints"], endpointApi.list)`）；CRUD 用 `useMutation`+toast+`invalidateQueries`；表单↔JSON 双视图（`EndpointForm` 内 Tabs + 双向同步：改 form 同步 `JSON.stringify`，改 JSON 解析回写 form）。
- 删除是卡片内联图标按钮（`EndpointCard.tsx:194-196`），**无右键菜单、无确认 Dialog**。

## 3. 状态管理
- stores barrel `src/stores/index.ts`，模块在 `stores/modules/`：`useLayoutStore`(activeView/导航/语言，persist 到 `layout-prefs`)、`useFilterStore`、`useProxyStore`、`useUpdateStore`。
- 约定：服务端数据走 react-query；跨页持久偏好走 zustand persist；页面级 UI 态用 useState。
- hooks：useEndpoints / useEndpointHealth / useEndpointHealthEvents / useStats / useUpdate / 主题 / 托盘。

## 4. Service 层（invoke 封装）
- `src/services/request.ts`：`request<T>(command, args)` 包 `invoke`，错误归一成 `Error`。约定：命令名 snake_case，参数键 camelCase（Tauri 自动转 snake），事件名 kebab-case 经 `subscribe()` + `Events` 常量。
- 模块 `services/modules/*` 按后端 commands 垂直切分，barrel `services/index.ts`。
- 页面错误处理惯例：`toast.error(e instanceof Error ? e.message : String(e))`。

## 5. CodeMirror 编辑器
- 仅 `pages/Endpoints/_components/JsonEditor.tsx`（lazy）：`@uiw/react-codemirror` + `@codemirror/lang-json`，固定 height、`basicSetup`。
- **缺**：格式化按钮、只读切换、TOML 语言包、共享提升。需求要这些 → 建议提升到 `src/components/common/JsonEditor.tsx` 增 `readOnly/onFormat/height` props。
- vite.config.ts 已把 CodeMirror 拆 `editor-vendor` chunk。

## 6. UI 组件库（shadcn/radix 封装于 `src/components/ui/`）
- 现成：Tabs / Button / Input / Switch / Dialog / Select / DropdownMenu / Sonner(Toaster) / ScrollArea / Popover / HoverCard / Badge / Card / Label / Separator / Tooltip。
- **无 ContextMenu 组件**（全库无 onContextMenu）；DropdownMenu 已封装但未被使用。删除渠道可：新增 radix context-menu / 复用 DropdownMenu(⋯) / 内联按钮+Dialog 确认。
- 空态：`components/common/Placeholder.tsx`。Tab 受控范本：`pages/Statistics/index.tsx:14-36`。

## 7. 后端 Rust 结构
- 顶层 `src-tauri/src/`：`lib.rs`(run + invoke_handler 注册 + setup)、`error.rs`(AppError/AppResult，序列化为字符串)、`state.rs`(AppState: db_pool/proxy/stats)、`commands/`(薄层)、`models/`、`modules/`(业务)、`utils/`。
- 已注册命令（`lib.rs:116-164`）：health/proxy/stats/usage/backup/config/endpoint/models/tokens/logs/webdav/window/update。**无** Claude/Codex 配置文件读写命令。
- 命令模式：`commands/endpoint.rs:18-22` 取 `state.db_pool` → 调 repo。

## 8. 后端文件 IO 现状
- Cargo 依赖：`serde/serde_json`✅、`rusqlite + r2d2_sqlite`✅、chrono/uuid/reqwest/axum；**无 `toml`、无 `dirs`**（需新增）。
- 路径工具 `utils/paths.rs`（27 行）：`app_data_dir(app)`、`db_path(app)`、`home_dir()`(USERPROFILE/HOME)。
- 已有读本机目录逻辑：`modules/usage_local/mod.rs:60-68` 只读 `~/.claude/projects/**/*.jsonl` 与 `~/.codex/sessions`，**不读 settings.json/config.toml**。
- **无通用 atomic_write** → 需新建 `utils/atomic_write.rs`（temp + rename）。
- 应用自身配置：SQLite `app_config` KV 表 + 迁移 v1-v8（当前 v8=request_logs.actual_model）。

## 9. i18n
- `src/lib/i18n.ts` `translate(lang,key)` + `useTranslation()`；`locales/zh.ts`/`en.ts` 仅 nav.* + common.*。
- 导航走 `label/labelEn`，**页面文案基本硬编码中文**；`useTranslation` 未被任何页面使用。新页面短期硬编码中文即可（与现状一致）。

## 10. 测试约定
- 前端 Vitest（jsdom，`src/**/*.{test,spec}.{ts,tsx}`），`src/test/setup.ts` mock 了 `invoke`/`listen`。现有测试偏纯函数/小组件。
- Rust：`#[cfg(test)] mod tests` 放在 modules 内，测 repo/parse/merge；命令层一般不单测。
- 新功能建议测：Rust 操作字段 merge / 非操作字段 diff / TOML↔JSON / 原子写入（temp dir）；前端 merge/preview 纯函数 + JsonEditor 格式化/只读。

## 11. 新页面推荐落点
### 前端新增
- `pages/ConfigProfiles/index.tsx` + `_components/{ChannelList,ChannelForm,OperFieldsEditor,MergedConfigEditor,ApplyBar}.tsx`
- `components/common/JsonEditor.tsx`（提升+增强）
- `services/modules/tool_config.ts`、`hooks/useToolConfigChannels.ts`、（可选）`stores/modules/configProfiles.ts`
### 前端修改
- `stores/modules/layout.ts`(ViewId)、`layouts/navConfig.tsx`、`layouts/AppLayout.tsx`、`services/index.ts`、（可选）locales。
### 后端新增
- `commands/tool_config.rs`、`modules/tool_config/{mod,claude,codex}.rs`、`models/tool_config.rs`、`utils/atomic_write.rs`、扩展 `utils/paths.rs`。
- 依赖：`toml`（Codex），可选 `dirs`。
### 后端修改
- `lib.rs`(注册命令)、`commands/mod.rs`、`modules/mod.rs`、`models/mod.rs`、`Cargo.toml`。
### 存储二选一
1. 纯文件（与需求文档一致）：`app_data_dir/profiles/claude_code/<渠道>/...`，无需迁移；
2. SQLite v9 索引表 + 文件落盘。
### 可复用能力
- 端点列表 `endpointApi.list()`；网关 port（Settings 读 `configApi.getConfig().port`）；对外模型 `advertisedModels()`；Toast/Dialog/Tabs/Switch；Rust `paths::home_dir()`。
