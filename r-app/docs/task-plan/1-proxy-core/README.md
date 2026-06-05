# 阶段 1：核心代理与轮换

> 任务总线：[../../TASKS.md](../../TASKS.md) ｜ 规划索引：[../README.md](../README.md)
> 起点参考：旧版 ccNexus 源码 `docs/origin/ccNexus/`（Go + Wails，仅参考不复用）

## 阶段目标

在 Rust 侧用 axum 起本地 HTTP 代理，实现端点轮换、故障转移、重试策略、端点解析（头部/模型名/查询参数）、上游转发与活跃请求管理，并提供启停/状态/手动切换命令与前端控制。与阶段 2 共同交付 **里程碑 M2 代理可用**。

## 前置依赖

- 阶段 0（骨架/基建）：`AppState`、`AppError`、存储、`lib.rs` 注册中心。
- 端点数据模型（P1-1）依赖 P0-4 的库表。

## 任务清单

### P1-1 端点与凭证数据模型 + 仓库
- 所属层：Rust
- 文件：`src-tauri/src/models/endpoint.rs`、`src-tauri/src/modules/storage/endpoint_repo.rs`
- 实现要点：`Endpoint`（id, name, api_url, api_key, auth_mode, enabled, transformer, model, remark, sort_order, test_status, created_at, updated_at）、`EndpointCredential`。`endpoint_repo.rs` 提供参数化查询 CRUD（`?` 占位防注入）、按 `sort_order` 排序读取、`list_enabled()`。
- 前置：P0-4
- 验收：插入/查询/更新/删除端点单测通过。
- PRD Story：1, 30

### P1-2 端点解析器
- 所属层：Rust
- 文件：`src-tauri/src/modules/proxy/resolver.rs`、`src-tauri/src/models/proxy.rs`
- 实现要点：`resolve_endpoint(headers, model, query)` 实现三种指定方式：① HTTP 头部 `X-CCmomo-Endpoint`；② 模型名 `@端点名/模型名` 解析；③ 查询参数。优先级与旧版 `endpoint_resolver.go` 对齐。返回是否使用「指定端点」标志（影响是否轮换）。
- 前置：P1-1
- 验收：三种解析方式单测覆盖；非法格式回退到默认轮换。
- PRD Story：6, 7, 8

### P1-3 端点轮换与重试策略
- 所属层：Rust
- 文件：`src-tauri/src/modules/proxy/rotation.rs`
- 实现要点：线程安全轮换器（`Mutex`/原子）：顺序循环 `current = (old + 1) % n`；最大重试 = 端点数 × 2；同一端点连续失败 2 次后切换；网络瞬时错误重试同一端点（300ms 延迟）；手动切换接口。区分「指定端点」时不轮换。
- 前置：P1-2
- 验收：模拟连续失败的单测验证 2 次后切换、循环回绕、瞬时错误重试延迟。
- PRD Story：2, 3, 4, 5

### P1-4 上游转发、连接池与活跃请求管理
- 所属层：Rust
- 文件：`src-tauri/src/modules/proxy/forward.rs`
- 实现要点：用 `reqwest::Client`（配置 `pool_max_idle_per_host`、超时 300s、TLS 握手超时等，对齐旧版连接池参数）转发请求；跟踪每个端点的活跃请求计数；切换端点前等待活跃请求完成（story 9）；手动切换时取消旧端点的进行中请求（`CancellationToken`，story 10）。
- 前置：P1-3
- 验收：转发成功返回上游响应；切换时活跃请求统计正确；取消逻辑有单测/集成测试。
- PRD Story：9, 10, 性能（连接池）

### P1-5 axum 代理服务启停与路由
- 所属层：Rust
- 文件：`src-tauri/src/modules/proxy/server.rs`、`src-tauri/src/modules/proxy/mod.rs`
- 实现要点：用 axum 在本地端口（来自 app_config，默认 3000）起服务，路由：`/`（主代理）、`/v1/messages/count_tokens`、`/v1/models`、`/health`、`/stats`。`ProxyHandle` 持有 `JoinHandle` + 关停信号，供 `AppState` 管理；提供 `start/stop/restart`。
- 前置：P1-4
- 验收：可绑定端口并响应 `/health`；停止后端口释放。
- PRD Story：1, 2

### P1-6 代理命令（启停/状态/手动切换）
- 所属层：Rust
- 文件：`src-tauri/src/commands/proxy.rs`，注册到 `lib.rs`
- 实现要点：`start_proxy/stop_proxy/get_proxy_status/switch_endpoint(name)`。命令薄，调 `modules::proxy`。状态变更通过事件 `proxy-status-changed` 推送前端。
- 前置：P1-5
- 验收：前端可启停代理并收到状态事件。
- PRD Story：1, 2, 10

### P1-7 代理控制前端
- 所属层：React
- 文件：`src/stores/modules/proxy.ts`、`src/pages/Dashboard/index.tsx`、`src/services/modules/proxy.ts`（proxyApi）
- 实现要点：Dashboard 顶部用 shadcn Card + Switch 控制代理启停，Badge 显示运行状态与当前端点；监听 `proxy-status-changed` 事件更新 Zustand；操作结果用 sonner 提示。
- 前置：P1-6, P0-8
- 验收：点击开关可启停代理，状态实时反映，错误有 toast。
- PRD Story：1, 2, 10

## 参考材料（origin）

> 路径相对 `docs/origin/ccNexus/`

| 关注点 | 文件 | 关键符号 / 说明 |
|--------|------|-----------------|
| 轮换 / 当前端点 / 手动切换 | `internal/proxy/proxy.go` | `rotateEndpoint`、`getCurrentEndpoint`、`SetCurrentEndpoint`；`currentIndex=(old+1)%n` |
| 请求处理 / 重试策略 | `internal/proxy/proxy_request.go` | `handleProxyRequest`、`runEndpointAttempt`、`handleSendError`；连续失败 2 次切换、瞬时错误 300ms 重试 |
| 端点解析（头/模型/查询） | `internal/proxy/endpoint_resolver.go` | `ResolveEndpoint`、`parseEndpointFromHeader`、`parseEndpointFromModel` |
| 请求准备 / 响应 / 入口 | `internal/proxy/{request,response,handler}.go` | 请求构造、上游响应处理 |
| 连接池 / 工具 | `internal/proxy/utils.go` | 连接池参数、超时 |
| Endpoint 结构 | `internal/config/config.go` | 字段对齐 |

## 完成判据

- 代理可在配置端口启动并响应 `/health`；停止后端口释放；
- 轮换/重试/解析三类单测通过（详见阶段 10 P10-3）；
- 前端可一键启停代理并实时反映状态。
