# 阶段 0：项目骨架与基建

> 任务总线：[../../TASKS.md](../../TASKS.md) ｜ 规划索引：[../README.md](../README.md)
> 起点参考：旧版 ccNexus 源码 `docs/origin/ccNexus/`（Go + Wails，仅参考不复用）

## 阶段目标

搭起 Tauri 2 + Rust 后端骨架：补齐依赖、统一错误类型、全局 `AppState`、SQLite(WAL) + 自动迁移、设备唯一 ID、`lib.rs` 注册中心与日志、IPC 约定与前端封装、应用骨架布局（已落地）。交付 **里程碑 M1 可运行骨架**。

## 前置依赖

无（项目起点）。本阶段是后续所有阶段的地基。

## 任务清单

### P0-1 后端 Cargo 依赖补齐
- 所属层：Rust
- 文件：`src-tauri/Cargo.toml`
- 实现要点：在现有 `tauri / tauri-plugin-opener / serde / serde_json` 基础上 `cargo add`：`thiserror`、`tokio --features full`、`reqwest --features json,stream`、`rusqlite --features bundled`、`r2d2`、`r2d2_sqlite`、`chrono --features serde`、`uuid --features v4`、`tracing`、`tracing-subscriber`、`axum`、`hyper`、`tower`、`tower-http`、`reqwest_dav`、`futures`、`async-trait`。`tauri` 启用 `tray-icon`、`image-png` feature。
- 前置：无
- 验收：`cargo check` 通过，无未使用告警导致失败。
- PRD Story：基建（支撑 1-80）

### P0-2 统一错误类型 AppError
- 所属层：Rust
- 文件：`src-tauri/src/error.rs`
- 实现要点：`thiserror` 定义 `AppError`，含 `Db`、`Network(#[from] reqwest::Error)`、`Io(#[from] std::io::Error)`、`Json(#[from] serde_json::Error)`、`Proxy(String)`、`Transform(String)`、`WebDav(String)`、`NotFound(String)`、`InvalidArgument(String)`、`Config(String)`、`Unknown(String)`；实现 `Serialize`（序列化为字符串），供前端 catch。定义 `pub type AppResult<T> = Result<T, AppError>`。
- 前置：P0-1
- 验收：`cargo check` 通过；任意命令返回 `Err` 时前端可在 catch 中拿到中文消息。
- PRD Story：基建

### P0-3 全局状态 AppState
- 所属层：Rust
- 文件：`src-tauri/src/state.rs`
- 实现要点：`AppState` 持有 `db_pool: r2d2::Pool<SqliteConnectionManager>`、`proxy: Mutex<Option<ProxyHandle>>`（运行句柄）、`models_cache: Mutex<ModelsCache>`、`device_id: String`、`app_handle: OnceCell<AppHandle>`（用于事件推送）。提供 `new()` 在启动时构造（初始化 DB + 设备 ID）。
- 前置：P0-2
- 验收：`lib.rs` 中 `.manage(AppState::new(...))` 编译通过。
- PRD Story：26-30（存储基础）、24（事件推送基础）

### P0-4 SQLite 初始化、WAL 与自动迁移
- 所属层：Rust
- 文件：`src-tauri/src/modules/storage/db.rs`、`migration.rs`、`mod.rs`，`src-tauri/src/utils/paths.rs`
- 实现要点：`paths.rs` 解析数据目录（`AppHandle.path().app_data_dir()`，DB 命名 `ccnexus.db`）。`db.rs` 用 `r2d2_sqlite` 建池，连接初始化执行 `PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000; PRAGMA foreign_keys=ON;`。`migration.rs` 建表：`endpoints`、`endpoint_credentials`、`credential_usage`、`daily_stats`、`app_config`，并建 `schema_version` 表实现幂等版本迁移（参考旧版 `idx_daily_stats_date/endpoint/device` 索引、`UNIQUE(endpoint_name,date,device_id)`）。
- 前置：P0-3
- 验收：首次启动自动建库建表；重复启动不报错；`PRAGMA journal_mode` 返回 `wal`。
- PRD Story：26, 27, 28, 30

### P0-5 设备唯一标识
- 所属层：Rust
- 文件：`src-tauri/src/modules/storage/device.rs`
- 实现要点：`get_or_create_device_id()`：先查 `app_config` 中 `device_id`，无则用 `uuid::Uuid::new_v4()` 生成并写入持久化。供统计写入与 WebDAV 过滤使用。
- 前置：P0-4
- 验收：多次启动返回同一 ID；DB 中存在该值。
- PRD Story：29

### P0-6 lib.rs 注册中心与日志初始化
- 所属层：Rust
- 文件：`src-tauri/src/lib.rs`、`src-tauri/src/main.rs`、各 `mod.rs`
- 实现要点：声明 `mod commands/models/modules/utils/error/state`；`run()` 内初始化 `tracing-subscriber`、构造并 `manage(AppState)`、注册插件（opener，后续阶段追加 updater/process）、`setup` 钩子保存 `AppHandle` 到 state 并构建托盘（占位）、`invoke_handler!` 集中注册所有命令（随阶段增量追加）。`main.rs` 保持仅 `r_app_lib::run()`。
- 前置：P0-3
- 验收：`pnpm tauri dev` 可启动空壳应用，无 panic。
- PRD Story：基建

### P0-7 IPC 约定与前端服务层骨架
- 所属层：Rust + React
- 文件：`src/services/{request.ts,index.ts}`、`src/services/modules/*`、`src/lib/i18n.ts`、`src/locales/{zh,en}.ts`，约定文档段落
- 实现要点：约定命名规范——命令用 `snake_case`，参数对象键用 `camelCase`（Tauri 自动转换），事件名用 `kebab-case`（如 `stats-updated`、`proxy-status-changed`、`log-line`、`update-progress`）。**服务层**（见 TASKS.md §3.2/§3.3）：`services/request.ts` 封装 `invoke<T>` + `AppError` 归一 + `listen` 事件订阅辅助；`services/modules/{domain}.ts` 按领域（对齐后端 `commands/`）导出 `xxxApi` 对象；`services/index.ts` 做 barrel 聚合。本阶段先落 `request.ts` + barrel + 领域桩，各领域方法随对应阶段补全。`i18n.ts` 实现轻量 `t(key)` + `setLanguage` + 持久化（localStorage + 后端 config 双写），资源置 `src/locales/{zh,en}.ts`。
- 前置：P0-6
- 验收：前端可经 `services` 成功调用一个示例命令（如 `health::get_health` 占位）并解析返回。
- PRD Story：59-61（i18n 基础）、基建

### P0-8 应用骨架布局与无边框标题栏
- 所属层：React + 配置
- 文件：已落地 `src/layouts/{AppLayout,TopNav,SideNav,NavItem,Logo,LangToggle,navConfig}.tsx`、`src/stores/layout.ts`、`src/components/Placeholder.tsx`、`src/pages/Dashboard.tsx`、`src/App.tsx`；待完成 `src/layouts/TitleBar.tsx`、`WindowControls.tsx`、`src-tauri/tauri.conf.json`
- 实现要点：
  - **【已落地，详见 [`../../LAYOUT.md`](../../LAYOUT.md)】** 单一 `AppLayout` 双形态导航：水平顶部 / 垂直侧边（展开 220px ↔ 折叠 56px）。状态 `navMode/sidebarState/activeView/lang` 存 Zustand + persist（localStorage `layout-prefs`）；6 个页面用 `activeView` 视图切换（保持 KISS，不引 router）；折叠态用 shadcn Tooltip 显示导航名；响应式 ≤1024 自动折叠；NavItem 激活态按 DESIGN token（horizontal = primary pill 黑字；vertical = `primary/12` 底 + `primary-soft` 字 + primary 图标）。
  - **【待完成】** 无边框窗口：`tauri.conf.json` 设 `"decorations": false`；自定义 `TitleBar`（`data-tauri-drag-region` 拖拽 + 双击最大化）+ `WindowControls`（最小化/最大化/关闭，调用 `getCurrentWindow().minimize()/toggleMaximize()/close()`，关闭行为走 P6-2）；所需 `core:window` 权限见 P11-2。当前顶栏已挂 `data-tauri-drag-region` 占位。
  - **【架构基线】** 前端目录架构（`pages/{View}/index.tsx` + `_components/`、`components/{ui,common,business}/`、`services/`、`stores/modules/`、`locales/`）以 TASKS.md §3.2/§3.3 为准；已落地文件的目录形态迁移与 Vite 模板脚手架清理见 **P0-9**。
- 前置：P0-7
- 验收：六个页面可切换（已）；顶部/侧边形态可切换并持久化（已）；侧边可折叠、折叠态有 Tooltip（已）；导航图标用 lucide-react、明暗主题正常（已）；窗口无原生边框且自定义标题栏可拖拽/最小化/最大化/关闭（待完成）。
- PRD Story：前端基础（支撑 44, 45, 59, 62 等）

### P0-9 前端目录架构落地与脚手架清理
- 所属层：React
- 文件：`src/components/{ui,common,business}/index.ts`、`src/stores/{modules/,index.ts}`、`src/pages/{View}/index.tsx`；迁移已落地 `StatusDot`/`TabularText`(→`components/ui/`)、`ThemeToggle`(→`components/common/`)、`stores/layout.ts`(→`stores/modules/layout.ts`)、`pages/Dashboard.tsx`(→`pages/Dashboard/index.tsx`)；删除 Vite 模板 demo（`components/{Counter,TodoList,UseStateDemo}.tsx`、`stores/{counter,counterContext,todo}.ts`）
- 实现要点：按 TASKS.md §3.2/§3.3 建立前端分层骨架——① `components/` 三级分类（`ui`/`common`/`business`）各加 `index.ts` barrel，将已落地视觉基元（StatusDot/TabularText）归入 `ui/`、系统功能组件（ThemeToggle，及后续 LangToggle/Logo）归入 `common/`；② `stores/` 改为 `stores/modules/{name}.ts` + `stores/index.ts` barrel（迁移 `layout.ts` 并同步 `@/stores` 导入）；③ 页面统一为 `pages/{View}/index.tsx` + 私有 `_components/`（先建占位 index.tsx，私有模块随各前端阶段补全）；④ 删除 Vite / UI-stack 模板遗留 demo 文件与 store，更新 `App.tsx` / `main.tsx` 引用。`services/` 骨架已在 P0-7 建立。
- 前置：P0-7, P0-8
- 验收：`pnpm build` 通过且浏览器明暗双主题验证无回归；无 demo 残留；`components/{ui,common,business}` 与 `stores` 均可经 barrel 导入；六页面 `pages/{View}/index.tsx` 可切换。
- PRD Story：前端基础（支撑全部前端阶段）

## 参考材料（origin）

> 路径相对 `docs/origin/ccNexus/`

| 关注点 | 文件 | 说明 |
|--------|------|------|
| 配置结构（Endpoint / AppConfig 字段、safeConfigKeys 雏形） | `internal/config/config.go` | 字段命名与默认值参考 |
| 建表 SQL / WAL PRAGMA / 迁移 / 索引 | `internal/storage/sqlite.go` | `idx_daily_stats_*`、`UNIQUE(endpoint,date,device)` |
| 存储接口抽象 | `internal/storage/interface.go` | 仓库接口分层思路 |
| 适配器 | `internal/storage/adapter.go`、`stats_adapter.go` | DB ↔ 模型转换 |
| 日志初始化 / 级别 | `internal/logger/logger.go` | 对标 `tracing-subscriber` |
| 单实例（可选） | `internal/singleinstance/singleinstance_windows.go` | 对标 `tauri-plugin-single-instance` |

## 完成判据（里程碑 M1）

- `pnpm tauri dev` 启动空壳应用无 panic；
- 首次启动自动建库建表，`PRAGMA journal_mode` 返回 `wal`；设备 ID 持久化且多次启动一致；
- 前端可经 `services` 成功调用一个占位命令并解析返回；
- 应用骨架布局（顶部 / 侧边 / 折叠三态）可切换并持久化；
- 前端目录架构骨架就位（`pages/{View}/index.tsx` + `_components/`、`components/{ui,common,business}` barrel、`services/`、`stores/modules/`），无模板 demo 残留。
