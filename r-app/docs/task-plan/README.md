# ccNexus 任务拆分计划（task-plan）

> 本目录是 [`../TASKS.md`](../TASKS.md)（**任务总线**）的落地拆分。
> TASKS.md 只保留共享上下文与阶段索引；**每个阶段的可执行任务明细与参考材料都在本目录的子目录里**。

---

## 一、起点来源（务必先读）

所有任务的起点参考是旧版 ccNexus 实现，位于 [`../origin/`](../origin/)：

| 资料 | 路径 | 用途 |
|------|------|------|
| 旧版完整源码 | `docs/origin/ccNexus/`（Go + Wails v2 + 原生 JS） | 实现要点的**参考样本**，不直接复用 |
| 产品需求 | `docs/origin/PRD.md` | 80 条 User Story 的权威来源 |
| 功能→代码索引 | `docs/origin/FEATURE_IMPLEMENTATION_INDEX.md` | 功能到旧版文件/函数的映射 |

> 重构目标技术栈为 **Tauri 2（Rust）+ React 19**；旧版 Go 代码仅用于理解算法与边界，**架构按 `TASKS.md` 第三章重新设计**。

---

## 二、目录与命名规范

```
docs/
├── TASKS.md                 ← 任务总线（共享上下文 + 阶段索引）
├── DESIGN.md / LAYOUT.md    ← 设计语言 / 布局规范（前端基线）
├── origin/                  ← 旧版参考资料（起点来源）
└── task-plan/
    ├── README.md            ← 本文件（拆分规划索引）
    └── {阶段号}-{slug}/
        └── README.md        ← 该阶段的任务明细 + origin 参考材料
```

- 子目录命名：`{阶段号}-{slug}`，阶段号与 TASKS.md 的「阶段 N」一致（0–11）。
- 每个阶段 README 统一结构：**阶段目标 → 前置依赖 → 任务清单（Pn-x 明细）→ 参考材料（origin 映射）→ 完成判据**。
- 任务编号沿用 `P{阶段}-{序号}`，与总线一一对应。
- **前端 `src/` 目录架构规范**（页面 + `_components` 双层、`components` 三级分类 `ui/common/business`、`services` 与 `stores` 分 `modules`、命名/barrel 约定）见 [`../TASKS.md` §3.2/§3.3](../TASKS.md)，源参考 [`../soybean-architecture-analysis目录规范.html`](../soybean-architecture-analysis目录规范.html)。各阶段前端任务的文件路径均以此为准。

---

## 三、阶段索引

| 阶段 | 目录 | 主题 | 任务 | 里程碑 |
|------|------|------|------|--------|
| 0 | [`0-bootstrap`](./0-bootstrap/README.md) | 项目骨架与基建 | P0-1 ~ P0-9 | M1 可运行骨架 |
| 1 | [`1-proxy-core`](./1-proxy-core/README.md) | 核心代理与轮换 | P1-1 ~ P1-7 | M2 代理可用 |
| 2 | [`2-transform`](./2-transform/README.md) | API 格式转换 | P2-1 ~ P2-6 | M2 代理可用 |
| 3 | [`3-storage-stats`](./3-storage-stats/README.md) | 存储层与统计 | P3-1 ~ P3-7 | M3 数据闭环 |
| 4 | [`4-config-endpoint-backend`](./4-config-endpoint-backend/README.md) | 配置与端点管理后端 | P4-1 ~ P4-9 | M3 数据闭环 |
| 5 | [`5-webdav`](./5-webdav/README.md) | WebDAV 同步 | P5-1 ~ P5-5 | M4 同步与体验 |
| 6 | [`6-tray-theme-i18n`](./6-tray-theme-i18n/README.md) | 托盘 / 主题 / 多语言 | P6-1 ~ P6-5 | M4 同步与体验 |
| 7 | [`7-endpoints-ui`](./7-endpoints-ui/README.md) | 端点管理前端 | P7-1 ~ P7-5 | M5 完整前端 |
| 8 | [`8-models-health-token-ui`](./8-models-health-token-ui/README.md) | 模型 / 健康 / Token 前端 | P8-1 ~ P8-3 | M5 完整前端 |
| 9 | [`9-auto-update`](./9-auto-update/README.md) | 自动更新 | P9-1 ~ P9-3 | M5 完整前端 |
| 10 | [`10-testing`](./10-testing/README.md) | 测试 | P10-1 ~ P10-6 | M6 发布就绪 |
| 11 | [`11-release`](./11-release/README.md) | 打包与发布 | P11-1 ~ P11-3 | M6 发布就绪 |

---

## 四、origin 参考材料总映射

各阶段需重点研读的旧版源码（路径相对 `docs/origin/ccNexus/`）：

| 阶段 | 主要 origin 参考 |
|------|------------------|
| 0 骨架 | `internal/config/config.go`、`internal/storage/{sqlite,interface,adapter}.go`、`internal/logger/logger.go`、`internal/singleinstance/*` |
| 1 代理 | `internal/proxy/{proxy,proxy_request,endpoint_resolver,request,response,handler,utils}.go` |
| 2 转换 | `internal/transformer/{transformer,types,registry,tool_chain}.go`、`internal/transformer/convert/{claude_openai,common,think_tags}.go`、`internal/proxy/streaming.go` |
| 3 统计 | `internal/proxy/stats.go`、`internal/service/{stats,archive}.go`、`internal/storage/{stats_adapter,credential_usage}.go` |
| 4 配置/端点 | `internal/service/{endpoint,settings}.go`、`internal/config/config.go`、`internal/proxy/{models,utils}.go`、`internal/tokencount/{estimator,image}.go` |
| 5 WebDAV | `internal/webdav/{client,sync,types}.go`、`internal/service/{webdav,backup,backup_helpers,backup_local,backup_types}.go` |
| 6 托盘/主题/i18n | `internal/tray/*`、`internal/service/settings.go`、`cmd/desktop/app.go`、`cmd/desktop/frontend/src/i18n/*` |
| 7 端点前端 | `cmd/desktop/frontend/src/modules/{endpoints,filters,modal,history}.js`、`cmd/server/webui/ui/js/components/endpoints.js` |
| 8 模型/健康/Token 前端 | `cmd/desktop/frontend/src/modules/stats.js`、`cmd/server/webui/ui/js/components/{dashboard,stats}.js`、`docs/models_api.md` |
| 9 更新 | `internal/updater/{updater,downloader,github,version,apply_windows,apply_other}.go`、`cmd/desktop/frontend/src/modules/updater.js`、`.github/workflows/build.yml` |
| 10 测试 | `internal/proxy/{request_test,streaming_usage_test,token_extraction_test}.go`、`internal/transformer/convert/*_test.go` |
| 11 发布 | `cmd/desktop/wails.json`、`.github/workflows/build.yml`、`docs/development.md` |

---

## 五、推荐推进顺序

依赖关系详见 [`../TASKS.md` §五](../TASKS.md)。建议路径：

```
阶段0 → (阶段1 → 阶段2) ┐
       → 阶段3          ├→ 阶段10 测试
       → 阶段4 → 阶段5  ┘
阶段0/4 → 阶段6
阶段4/5 → 阶段7 / 阶段8
阶段0 → 阶段9
全部 → 阶段11 发布
```

> Out of Scope（不创建任务）：Token Pool 管理、OAuth/MFA、高级统计（图表/导出/报表）、高级同步（冲突/增量/多版本）。

---

## 六、任务进度跟踪（progress.csv）

任务进度统一在 [`progress.csv`](./progress.csv) 跟踪（68 条任务，P0-1 ~ P11-3），**TASKS.md 与各阶段 README 保持稳定、不随进度频繁变更**；日常只更新本 CSV 的 `状态 / 负责人 / 开始日期 / 完成日期 / 备注` 列。

- 文件编码：UTF-8 with BOM（Excel / WPS 直接打开中文不乱码）。
- 列：`阶段, 里程碑, 任务编号, 标题, 所属层, 前置任务, PRD Story, 状态, 负责人, 开始日期, 完成日期, 备注`。
- `状态` 取值：`待开始` / `进行中` / `已完成` / `阻塞` / `已取消`。
- 任务的「实现要点 / 文件路径 / 验收标准」仍以对应阶段 README 为准；CSV 只承载状态，不复制明细。
- 新增/调整任务时：先在对应阶段 README 增改 `Pn-x` 明细，再在 CSV 追加同编号行（同步更新 TASKS.md §四 与 §三 的任务范围）。

> 日期建议 `YYYY-MM-DD`。开始任务时填 `开始日期` 并置 `进行中`；完成时填 `完成日期` 并置 `已完成`；受阻置 `阻塞` 并在 `备注` 写明原因/依赖。初始状态：P0-8 为 `进行中`（布局已落地、标题栏待完成），其余为 `待开始`。
