# ccNexus 重构开发任务文档（Tauri 2 + React 19）

> **本文件是任务总线（Task Bus）**：只承载跨阶段的共享上下文（技术栈、设计基线、目标架构、依赖、里程碑、决策）。
> **各阶段的可执行任务明细与参考材料已拆分到 [`task-plan/`](./task-plan/) 下**，按阶段一个目录。先读本文件建立全局认知，再进对应阶段目录执行。

## 一、文档说明

本项目将 `docs/origin/PRD.md` 中的 80 条 User Story 拆解为可执行开发任务，适配已确定的技术栈：**Tauri 2（Rust 后端）+ React 19（前端）**。

- **起点来源**：旧版 ccNexus 完整实现位于 [`docs/origin/ccNexus/`](./origin/ccNexus/)（Go + Wails + 原生 JS + SQLite），**仅作参考、不直接复用**；功能→旧版文件映射见 `docs/origin/FEATURE_IMPLEMENTATION_INDEX.md`，各阶段目录的「参考材料」小节已给出精确文件清单。
- **拆分结构**：任务明细在 [`task-plan/`](./task-plan/)，规划索引见 [`task-plan/README.md`](./task-plan/README.md)。
- **进度跟踪**：任务状态在 [`task-plan/progress.csv`](./task-plan/progress.csv) 跟踪（68 条任务，状态/负责人/日期列），本文件与各阶段 README 保持稳定、**不随进度频繁变更**。
- 后端组织范式：`commands/` 按领域垂直切分、`models/` 数据模型、`modules/` 业务逻辑、`utils/` 工具、统一 `error.rs`、`lib.rs` 作为命令注册中心（详见第三章）。
- 设计/布局基线：见 [`DESIGN.md`](./DESIGN.md) 与 [`LAYOUT.md`](./LAYOUT.md)，前端任务必须复用。
- 每个任务统一包含：任务编号、标题、所属层、文件路径、实现要点、前置任务、验收标准、回链 PRD Story 编号。全文相对路径以 `r-app` 项目根为基准。

### 范围边界（Out of Scope，不创建任务）

以下功能依据 PRD「Out of Scope」明确排除：Token Pool 管理（Codex Token 轮换/状态/刷新）、OAuth 与多因素认证、高级统计（实时图表/数据导出/自定义报表）、高级同步（冲突解决/增量同步/多版本管理）。

---

## 二、确定的技术栈

### 后端（`src-tauri/` / Rust）

| 领域 | 选型 | 说明 |
|------|------|------|
| 应用框架 | Tauri 2 | 桌面应用外壳、IPC、托盘、更新 |
| 本地代理服务 | axum + hyper + tower | 在 Rust 侧起本地 HTTP 代理，承载端点轮换/故障转移/重试 |
| 异步运行时 | tokio（features = full） | 代理、异步命令、定时任务 |
| HTTP 客户端 | reqwest（json, stream） | 转发上游请求、模型列表拉取、端点测试 |
| 数据库 | rusqlite（bundled）+ WAL，配合 r2d2 连接池 | 配置/统计持久化，支持并发读写 |
| 序列化 | serde / serde_json | 模型与 IPC |
| 错误处理 | thiserror（统一 `AppError`，实现 `Serialize`） | 全局错误类型 |
| 时间 | chrono（serde） | 四周期统计的日期计算 |
| WebDAV | reqwest_dav | 同步、备份、恢复、列表、删除、连接测试 |
| 设备标识 | uuid + machine 信息 | 设备唯一 ID |
| 日志 | tracing + tracing-subscriber | 分级日志、实时日志推送 |
| 系统托盘 | Tauri tray API（`tauri::tray`） | 托盘图标、菜单、窗口行为 |
| 自动更新 | tauri-plugin-updater + tauri-plugin-process | 检查/下载/安装更新 |
| 单窗口管理 | tauri-plugin-single-instance（可选） | 防止多开 |

### 前端（`src/` / React 19）— 已集成，禁止再创建安装/配置任务

以下库已完成集成并通过构建与浏览器验证，任务中直接引用使用：

- **样式**：Tailwind CSS 4（`@tailwindcss/vite`，CSS-first，token 写于 `src/index.css`）
- **组件库**：shadcn/ui（已生成于 `src/components/ui/`：button, card, input, label, dialog, select, badge, tabs, switch, separator, sonner, dropdown-menu, tooltip, scroll-area）+ Radix UI
- **图标 / 动画**：lucide-react、Motion（`motion/react`）
- **状态**：Zustand（客户端状态）、TanStack Query（封装 IPC/异步数据，已在 `main.tsx` 挂载 `QueryClientProvider`）
- **主题**：next-themes（已在 `main.tsx` 挂载 `ThemeProvider`，`attribute="class"`）
- **代码编辑器**：CodeMirror 6（`@uiw/react-codemirror` + `@codemirror/lang-json`），用于端点 JSON 配置编辑
- **IPC**：`@tauri-apps/api` 的 `invoke` + `event`
- **路径别名**：`@/` → `src/`；`@/lib/utils` 提供 `cn()`
- **通知**：sonner（`Toaster` 已挂载）
- **布局**：单一 `AppLayout` 双形态导航（顶部/侧边 + 折叠），已落地于 `src/layouts/`，详见 `LAYOUT.md`

### 设计系统基线（Dark Stripe，已落地，前端任务必须复用）

设计语言来源 `r-app/DESIGN.md`（dark-first「Unified Dark Stripe」，并已补齐适配浅底的 light 变体），已作为前期工作集成完毕并通过构建 + 明暗双主题浏览器验证，为下方全部前端任务提供视觉前提：

- **全局令牌**（`src/index.css`）：明暗双调色板（`:root` 亮色 / `.dark` 暗色，后者为 DESIGN.md 原生）——暗色真黑底 `#000`、亮色纯白底，各自四级灰阶表面（`surface-raised/card/hover/overlay`）、单一祖母绿 `#22c55e` 主色（CTA/链接/品牌/成功四用，含 `primary-soft/deep/press` 阶梯，浅底强调文字色已调深）、三阶 ink 文本（`ink-primary/secondary/mute/disabled`）、三级 edge 边线（`edge-subtle/edge/edge-strong`）、`shadow-level-1/2/3` + `shadow-focus-ring`、显式圆角标度（xs4/sm6/md8/lg12/xl16）、字体 `Inter Variable` + `JetBrains Mono Variable`（已 `@fontsource` 自带，离线可用）、`body` 全局 `ss01`、`.tabular` 工具类（mono + tnum）、全局滚动条（pill 浮动滑块、中性灰随主题切换）。
- **已对齐的 shadcn 组件**：Button/Badge → pill（`rounded-full`）；Button 默认绿底黑字 + hover `primary-soft` / active `primary-deep` + `cursor-pointer`，secondary/outline 带 `edge` 边；Badge 新增 `success/warning/info/danger/muted` 软语义变体（`pill-tag-soft` 风格）；Card → 12px 圆角且静态无阴影；Input → 6px 圆角 + `surface-raised` 内底。
- **签名组件**（视觉基元，归 `components/ui/`，经 `@/components/ui` barrel 导入；当前已落地于 `src/components/`，随 P0-9 迁入 `ui/`）：`StatusDot`（8px 语义圆点：success/danger/warning/info/idle，可 pulse）、`TabularText`（mono + tnum 数值）。
- **主题接线**：`main.tsx` 用 `ThemeProvider attribute="class" defaultTheme="system" enableSystem`（跟随系统、可手动切换），`@/components/ThemeToggle` 已启用；`next-themes` 切换 `<html>` 的 `light`/`dark` class，两套调色板自动生效（`ThemeToggle` 归 `components/common/`）。

**实施约束（贯穿阶段 6/7/8 等前端任务）**：① 一律引用 token（如 `bg-card`、`text-ink-secondary`、`border-edge`），禁止写死 hex；② 按钮一律 pill，CTA 用 `bg-primary text-primary-foreground`（黑字，不用 `text-white`）；③ 数值/计数/Token/时间戳一律 `TabularText` 或 `.tabular`；④ 状态指示用 `StatusDot` + 软 `Badge`（端点测试状态、代理在线、健康态）；⑤ 明暗双主题已就绪（`:root` 亮色 + `.dark` 暗色 + `system` 跟随 + `ThemeToggle`），主题任务 **P6-3/P6-4** 仅需实现主题选择持久化（写入 `app_config`）与按时间定时切换（Story 46-48），无需再建调色板。

---

## 三、目标架构概览

### 3.1 后端分层（`src-tauri/src/`）

```
src-tauri/src/
├── main.rs                      # 入口，调用 lib::run()
├── lib.rs                       # 注册中心：模块声明 + manage(AppState) + invoke_handler! + 插件 + setup(托盘)
├── error.rs                     # 统一 AppError（thiserror + Serialize）
├── state.rs                     # AppState：DB 连接池、代理句柄、模型缓存、配置缓存、设备 ID
│
├── commands/                    # 命令层（薄，垂直切分，仅解析参数 + 调 modules）
│   ├── mod.rs
│   ├── proxy.rs                 # 代理启停、状态、手动切换端点
│   ├── endpoint.rs              # 端点 CRUD、筛选、克隆、排序、测试
│   ├── stats.rs                 # 四周期统计查询、趋势、历史、月度归档/删除
│   ├── config.rs                # 应用配置读写（端口、日志级别、窗口行为等）
│   ├── webdav.rs                # WebDAV 配置、连接测试、备份/恢复/列表/删除、同步
│   ├── models.rs                # 模型列表查询/刷新
│   ├── health.rs                # 健康检查（脱敏端点）
│   ├── tokens.rs                # Token 计数估算
│   ├── logs.rs                  # 实时日志读取、日志级别设置
│   ├── update.rs                # 检查/下载/安装更新、跳过版本、更新设置
│   └── window.rs                # 窗口行为、托盘联动
│
├── models/                      # 数据模型（serde）
│   ├── mod.rs
│   ├── endpoint.rs              # Endpoint, EndpointCredential, Create/UpdateEndpointRequest
│   ├── stats.rs                 # DailyStat, PeriodStats, TrendCompare, CredentialUsage
│   ├── config.rs                # AppConfig, ThemeConfig, WindowBehavior, UpdateSettings
│   ├── webdav.rs                # WebDavConfig, BackupFile, BackupMeta, TestResult
│   ├── proxy.rs                 # ProxyStatus, HealthInfo, ModelInfo, ClientFormat
│   └── transform.rs            # ClaudeRequest, OpenAIRequest 等转换 DTO
│
├── modules/                     # 业务逻辑层
│   ├── mod.rs
│   ├── proxy/                   # 代理核心
│   │   ├── mod.rs
│   │   ├── server.rs            # axum 服务启停、路由注册
│   │   ├── rotation.rs          # 端点轮换 + 故障转移 + 重试策略（线程安全）
│   │   ├── resolver.rs          # header/model(@端点/模型)/query 端点解析
│   │   ├── forward.rs           # 上游转发、连接池、活跃请求跟踪、取消
│   │   └── streaming.rs         # SSE 流式响应处理
│   ├── transform/               # 格式转换
│   │   ├── mod.rs
│   │   ├── transformer.rs       # Transformer trait + 注册表（registry）
│   │   ├── claude_openai.rs     # Claude ↔ OpenAI Chat（含 tool_use / reasoning / stream）
│   │   └── types.rs             # 内部转换中间类型
│   ├── storage/                 # 存储
│   │   ├── mod.rs
│   │   ├── db.rs                # 连接池、WAL、PRAGMA
│   │   ├── migration.rs         # 建表 + 自动迁移 + 版本表
│   │   ├── endpoint_repo.rs     # 端点/凭证 CRUD
│   │   ├── stats_repo.rs        # daily_stats / credential_usage 读写、聚合
│   │   ├── config_repo.rs       # app_config 读写、safeConfigKeys
│   │   └── device.rs            # 设备 ID 获取/生成
│   ├── stats/                   # 统计聚合
│   │   ├── mod.rs
│   │   ├── periods.rs           # 四周期日期计算
│   │   ├── aggregator.rs        # 事件驱动统计 + 防抖保存（2s）+ 趋势对比
│   │   └── emitter.rs           # 通过 Tauri event 推送前端
│   ├── webdav/                  # WebDAV 同步
│   │   ├── mod.rs
│   │   ├── client.rs            # reqwest_dav 封装
│   │   └── sync.rs              # 备份/恢复/列表/删除、元数据、安全过滤
│   ├── models_cache.rs          # 模型列表缓存（30 分钟 TTL）+ 拉取
│   ├── tokens.rs                # Token 估算
│   └── tray.rs                  # 托盘构建、菜单、i18n 文案
│
└── utils/                       # 工具
    ├── mod.rs
    ├── mask.rs                  # API Key 脱敏
    ├── paths.rs                 # 数据目录、DB 路径解析
    └── time.rs                  # 时区/日期辅助
```

### 3.2 前端模块（`src/`）

> 目录架构参考 [`docs/soybean-architecture-analysis目录规范.html`](./soybean-architecture-analysis目录规范.html)（soybean-admin 的「页面 + modules 双层」与「components 三级分类」思路），按 React 生态与本项目特点（无 router、用 Zustand `activeView` 视图切换）适配。分层职责与命名约定见 §3.3。

```
src/
├── main.tsx                     # 已集成 Providers（保持不动，新增 i18n Provider）
├── App.tsx                      # 渲染 <AppLayout />（已落地）
├── index.css                    # Tailwind token + 明暗双调色板 + 滚动条
│
├── pages/                       # 路由页面（页面入口 index.tsx + _components 私有模块双层）
│   ├── Dashboard/               #（已落地占位 Dashboard.tsx，随 P0-9 改目录形态）
│   │   ├── index.tsx            # 组装层：编排 _components + 读顶层 store，无业务细节
│   │   └── _components/         # 页面私有模块（显式相对导入，不做 barrel）：HealthOverview …
│   ├── Endpoints/
│   │   ├── index.tsx
│   │   └── _components/         # EndpointCard / EndpointForm(CodeMirror) / FilterBar / CloneButton / TestBadge / DnDList / ModelList
│   ├── Statistics/
│   │   ├── index.tsx
│   │   └── _components/         # PeriodTabs / TrendBadge / EndpointStatsTable / HistoryPanel
│   ├── Sync/
│   │   ├── index.tsx
│   │   └── _components/         # WebdavForm / BackupList / ConnTest
│   ├── Logs/
│   │   └── index.tsx            # 简单页，必要时再加 _components/
│   └── Settings/
│       ├── index.tsx
│       └── _components/         # UpdateDialog / DownloadProgress / TokenCounter / 关闭行为·主题·语言设置区块
│
├── components/                  # 跨页面全局组件（三级分类，各层 barrel index.ts）
│   ├── ui/                      # shadcn 原子组件（已生成）+ 视觉基元 StatusDot / TabularText
│   ├── common/                  # 系统功能组件：ThemeToggle / LangToggle / Logo（无业务、Props 驱动；Logo/LangToggle 当前落地于 layouts/，建议迁入后由 layouts 导入）
│   └── business/                # 跨页面业务复合：StatCard（Statistics+Dashboard 共用）/ UpdateBadge（导航红点）
│
├── layouts/                     # 布局骨架（已落地）：AppLayout / TopNav / SideNav / NavItem / navConfig（+ 待加 TitleBar / WindowControls）
│
├── services/                    # IPC 服务层（对齐后端 commands/ 垂直切分）
│   ├── request.ts               # invoke<T> 封装 + AppError 归一 + listen 事件订阅辅助
│   ├── modules/                 # 按领域：endpoint / stats / proxy / config / webdav / models / health / tokens / logs / update / window
│   └── index.ts                 # barrel：聚合各领域 xxxApi
│
├── stores/                      # Zustand（modules 拆分 + barrel）
│   ├── modules/                 # layout(已落地，迁此) / proxy / filters / settings
│   └── index.ts                 # barrel：聚合各 store
│
├── hooks/                       # TanStack Query 数据封装（调 services）：useEndpoints/useStats/useWebdav/useModels/useUpdate/useLogs/useAutoTheme
│
├── locales/                     # i18n 资源：zh.ts / en.ts
│
└── lib/
    ├── utils.ts                 # cn()（已存在）
    └── i18n.ts                  # 轻量 i18n：t(key) + setLanguage + 持久化
```

### 3.3 前端分层职责与命名约定（soybean 适配）

**分层职责与复用边界**

| 层 | 位置 | 职责 | 复用范围 |
|----|------|------|----------|
| 页面入口 | `pages/{View}/index.tsx` | 组装：编排 `_components`、读顶层 store、布局排版，不含业务细节 | — |
| 页面私有模块 | `pages/{View}/_components/*.tsx` | 独立业务区块，自带数据/状态；仅服务所属页面 | 该页面私有 |
| UI 基元层 | `components/ui/` | shadcn 原子组件 + 无业务视觉基元（StatusDot/TabularText） | 全局 |
| 系统功能层 | `components/common/` | 系统级功能封装，Props 驱动、无副作用（主题/语言/Logo） | 全局 |
| 业务复合层 | `components/business/` | 跨页面业务复合体（StatCard/UpdateBadge） | 全局（跨页面） |
| 布局骨架 | `layouts/` | 顶/侧导航、折叠、标题栏，独立于业务 | 全局 |
| 服务层 | `services/` | IPC 调用按领域封装，对齐后端 `commands/` | 全局 |
| 状态层 | `stores/modules/` | Zustand 客户端状态（持久化偏好等） | 全局 |
| 数据层 | `hooks/` | TanStack Query 封装异步数据（内部调 services） | 全局 |

**归属判定**：组件只被单个页面用 → 放该页面 `_components/`；被多个页面或布局复用 → 提升到 `components/{ui|common|business}/`。不可复用的不进 `components/`。

**命名约定**

| 维度 | 约定 | 示例 |
|------|------|------|
| 组件文件名 | PascalCase | `EndpointCard.tsx` |
| 页面入口 | 目录 + `index.tsx` | `pages/Endpoints/index.tsx` |
| 页面私有模块 | `_components/`（下划线=私有，显式相对导入，无 barrel） | `import EndpointCard from './_components/EndpointCard'` |
| 全局组件导入 | 显式 import + 每层 `index.ts` barrel | `import { StatusDot } from '@/components/ui'` |
| Props | `interface Props` + 解构；camelCase | `({ endpoint, onTest }: Props)` |
| 事件回调 | `onXxx` Props（替代 Vue emit） | `onClone` / `onDelete` |
| 双向绑定 | `value` + `onChange`（替代 v-model） | `<Input value={v} onChange={setV} />` |
| Store | `useXxxStore`（`stores/modules/`） | `useLayoutStore` |
| 数据 hook | `useXxx`（`hooks/`） | `useEndpoints` |
| 服务对象 | `xxxApi`（`services/modules/{domain}.ts`） | `endpointApi.list()` |

**barrel 约定**：`components/{ui,common,business}/`、`services/`、`stores/` 各置 `index.ts` 聚合导出，模拟“按需使用”；`pages/*/_components/` **不做 barrel**（强调私有，一律显式相对路径导入），与全局组件形成“是否需要 import 别名”的天然区分。

---

## 四、阶段索引（任务明细见各目录）

> 任务编号规则：`P{阶段}-{序号}`。点击目录进入该阶段的任务明细 + origin 参考材料。

| 阶段 | 目录 | 主题 | 任务范围 | 所属层 | 里程碑 |
|------|------|------|----------|--------|--------|
| 0 | [`task-plan/0-bootstrap`](./task-plan/0-bootstrap/README.md) | 项目骨架与基建 | P0-1 ~ P0-9 | Rust + React | M1 |
| 1 | [`task-plan/1-proxy-core`](./task-plan/1-proxy-core/README.md) | 核心代理与轮换 | P1-1 ~ P1-7 | Rust + React | M2 |
| 2 | [`task-plan/2-transform`](./task-plan/2-transform/README.md) | API 格式转换 | P2-1 ~ P2-6 | Rust | M2 |
| 3 | [`task-plan/3-storage-stats`](./task-plan/3-storage-stats/README.md) | 存储层与统计 | P3-1 ~ P3-7 | Rust + React | M3 |
| 4 | [`task-plan/4-config-endpoint-backend`](./task-plan/4-config-endpoint-backend/README.md) | 配置与端点管理后端 | P4-1 ~ P4-9 | Rust | M3 |
| 5 | [`task-plan/5-webdav`](./task-plan/5-webdav/README.md) | WebDAV 同步 | P5-1 ~ P5-5 | Rust + React | M4 |
| 6 | [`task-plan/6-tray-theme-i18n`](./task-plan/6-tray-theme-i18n/README.md) | 托盘 / 主题 / 多语言 | P6-1 ~ P6-5 | Rust + React | M4 |
| 7 | [`task-plan/7-endpoints-ui`](./task-plan/7-endpoints-ui/README.md) | 端点管理前端 | P7-1 ~ P7-5 | React | M5 |
| 8 | [`task-plan/8-models-health-token-ui`](./task-plan/8-models-health-token-ui/README.md) | 模型 / 健康 / Token 前端 | P8-1 ~ P8-3 | React | M5 |
| 9 | [`task-plan/9-auto-update`](./task-plan/9-auto-update/README.md) | 自动更新 | P9-1 ~ P9-3 | Rust + React | M5 |
| 10 | [`task-plan/10-testing`](./task-plan/10-testing/README.md) | 测试 | P10-1 ~ P10-6 | Rust + React | M6 |
| 11 | [`task-plan/11-release`](./task-plan/11-release/README.md) | 打包与发布 | P11-1 ~ P11-3 | Rust | M6 |

---

## 五、任务依赖与里程碑

### 5.1 阶段依赖

```
阶段0（骨架/基建）
  ├─→ 阶段1（代理与轮换） ─┐
  ├─→ 阶段3（存储与统计） ─┤
  └─→ 阶段4（配置/端点后端）┤
阶段1 ─→ 阶段2（格式转换） ─┤
阶段2 + 阶段3 ─────────────→ 阶段10（测试：转换/统计/代理/集成）
阶段4 ─→ 阶段5（WebDAV） ──→ 阶段7（端点管理前端）/阶段8（模型/健康/Token 前端）
阶段0/4 ─→ 阶段6（托盘/主题/i18n）
阶段0 ─→ 阶段9（自动更新）
全部功能完成 ─→ 阶段11（打包发布）
```

### 5.2 里程碑

| 里程碑 | 包含阶段 | 交付标志 |
|--------|----------|----------|
| M1 可运行骨架 | 阶段0 | 应用启动、DB 建库、IPC 通路打通 |
| M2 代理可用 | 阶段1 + 阶段2 | 端点轮换 + Claude↔OpenAI 转换端到端可用 |
| M3 数据闭环 | 阶段3 + 阶段4 | 统计零延迟更新、端点管理后端、模型/健康/Token |
| M4 同步与体验 | 阶段5 + 阶段6 | WebDAV 同步、托盘、主题、i18n |
| M5 完整前端 | 阶段7 + 阶段8 + 阶段9 | 端点管理/统计/同步/更新全前端 |
| M6 发布就绪 | 阶段10 + 阶段11 | 测试通过、安装包产出 |

### 5.3 测试策略（对应 PRD Testing Decisions）

- **单元测试（Rust `#[cfg(test)]`）**：转换器（Claude↔OpenAI，含 tool_use/reasoning/stream/usage）、统计（周期/趋势/防抖）、存储（CRUD/迁移/聚合/归档）、轮换与端点解析。对应 P10-1/P10-2/P10-3。
- **集成测试（Rust `tests/`）**：代理端到端（mock 上游、故障转移、转换、统计写入）、WebDAV（备份/恢复/列表/删除/设备过滤）。对应 P10-4/P10-5。
- **前端交互测试（Vitest + Testing Library，mock IPC）**：端点筛选/克隆/测试/拖拽、统计事件刷新、更新红点、主题与语言切换。对应 P10-6。
- **边界情况**：网络瞬时错误、并发读写（WAL）、数据迁移幂等、配置同步设备过滤——分散覆盖于 P10-2~P10-5。

### 5.4 细节核对清单（确保不遗漏）

- i18n（zh/en）全界面覆盖 + 持久化：P6-5、P0-7
- 主题定时切换（7:00-19:00 / 19:00-7:00 可配）：P6-4
- API Key 脱敏：P4-7（后端）、P7-1（前端展示）
- 模型列表缓存 30 分钟：P4-6
- SQLite WAL + 并发：P0-4
- 统计防抖保存 2s：P3-3
- 统计事件驱动零延迟：P3-3、P3-6
- 端点筛选时禁用拖拽：P7-5
- WebDAV 同步设备特定配置过滤：P5-4、P4-1（safe_config_keys）
- 连接池优化：P1-4
- 网络瞬时错误重试 300ms / 连续失败 2 次切换：P1-3
- 备份元数据（时间/版本）：P5-3
- 更新红点 / 跳过版本 / 间隔配置 / 下载进度：P9-2、P9-3

### 5.5 决策记录与待确认

**已确认决策（已并入对应任务）**

1. **无边框自定义标题栏**：启用。`tauri.conf.json` 设 `"decorations": false`，实现自定义 `TitleBar`（`data-tauri-drag-region` 拖拽 + 最小化/最大化/关闭按钮），见 P0-8；所需 `core:window` 权限见 P11-2。
2. **端点指定的 HTTP 头名称**：`X-CCmomo-Endpoint`，见 P1-2。
3. **Token 计数精度**：沿用近似估算，暂不接入精确 tokenizer（如 tiktoken Rust 绑定），见 P4-8。

**待后续确认（暂留空，不阻塞结构性开发）**

4. **更新分发渠道**：自动更新的 `endpoints`（更新服务器地址）与签名 `pubkey` 暂留空，后续确定托管方再填入，见 P9-1。
5. **WebDAV 测试环境**：暂未提供正式测试端点，先以本地 stub（dufs / rclone serve webdav）占位，见 P10-5。
