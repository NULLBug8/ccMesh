# 阶段 4：应用配置与端点管理后端

> 任务总线：[../../TASKS.md](../../TASKS.md) ｜ 规划索引：[../README.md](../README.md)
> 起点参考：旧版 ccNexus 源码 `docs/origin/ccNexus/`（Go + Wails，仅参考不复用）

## 阶段目标

提供应用配置（端口/日志级别/语言/主题/窗口行为/更新/同步设置）的模型与命令，端点 CRUD/克隆/测试/排序命令，模型列表缓存（30 分钟），健康检查 + API Key 脱敏，Token 计数，实时日志。交付 **里程碑 M3 数据闭环** 的后端能力。

## 前置依赖

- 阶段 0（P0-4 库表、P0-2 错误、P0-6 注册）；
- 端点相关命令依赖 P1-1（端点模型/仓库）。

## 任务清单

### P4-1 应用配置模型与仓库
- 所属层：Rust
- 文件：`src-tauri/src/models/config.rs`、`src-tauri/src/modules/storage/config_repo.rs`
- 实现要点：`AppConfig`（port, log_level, language, theme, theme_auto, auto_light_theme, auto_dark_theme, close_window_behavior, update 设置, webdav 设置等）。`config_repo` 提供 `get_config/set_config` 与 `safe_config_keys`（同步白名单，参考旧版 `safeConfigKeys`，剔除设备特定项）。
- 前置：P0-4
- 验收：配置读写、白名单过滤单测通过。
- PRD Story：30, 39, 44, 45-48, 60, 72, 77

### P4-2 应用配置命令
- 所属层：Rust
- 文件：`src-tauri/src/commands/config.rs`，注册到 `lib.rs`
- 实现要点：`get_config/set_config/get_all_config`；修改端口/日志级别时联动代理重启与 tracing level；语言变更联动托盘重建。
- 前置：P4-1
- 验收：前端改端口后代理在新端口监听；改日志级别即时生效。
- PRD Story：44, 60, 77

### P4-3 端点 CRUD 与排序命令
- 所属层：Rust
- 文件：`src-tauri/src/commands/endpoint.rs`，注册到 `lib.rs`
- 实现要点：`list_endpoints(filter)`、`create_endpoint`、`update_endpoint`、`delete_endpoint`、`reorder_endpoints(ordered_ids)`（更新 `sort_order`）。后端做输入验证（URL/name 非空、唯一）。
- 前置：P1-1
- 验收：CRUD 与排序持久化；重复名拒绝。
- PRD Story：1, 69

### P4-4 端点克隆命令
- 所属层：Rust
- 文件：`src-tauri/src/commands/endpoint.rs`、`src-tauri/src/utils`（命名辅助）
- 实现要点：`clone_endpoint(id)`：复制端点，名称自动加副本后缀并避免冲突（`(副本)` / `(Copy)`，存在则追加序号，参考旧版 `extractBaseName`）。
- 前置：P4-3
- 验收：克隆生成新端点且名称不冲突。
- PRD Story：65, 66

### P4-5 端点测试命令
- 所属层：Rust
- 文件：`src-tauri/src/commands/endpoint.rs`、`src-tauri/src/modules/proxy/forward.rs`
- 实现要点：`test_endpoint(id)`：向端点发探测请求（如 `/v1/models` 或最小 chat），返回成功/失败与延迟；将 `test_status` 持久化到 `endpoints` 表（成功/失败/未测试）。
- 前置：P4-3
- 验收：可用端点返回成功并写入状态；不可用返回失败。
- PRD Story：67, 68

### P4-6 模型列表缓存与命令
- 所属层：Rust
- 文件：`src-tauri/src/modules/models_cache.rs`、`src-tauri/src/commands/models.rs`，注册到 `lib.rs`
- 实现要点：`ModelsCache`（数据 + updatedAt + ttl，默认 30 分钟，`RwLock`）；`fetch_models_from_endpoint`（多端点类型）；命令 `get_models(force_refresh)`：未过期取缓存，过期或强制刷新则拉取。参考旧版 `models.go`。
- 前置：P1-1
- 验收：30 分钟内重复调用命中缓存；`force_refresh` 重新拉取。
- PRD Story：49, 50, 51, 52, 性能（缓存）

### P4-7 健康检查与 API Key 脱敏命令
- 所属层：Rust
- 文件：`src-tauri/src/commands/health.rs`、`src-tauri/src/utils/mask.rs`
- 实现要点：`get_health()` 返回 `status`、`enabled_endpoints` 数量、脱敏端点列表；`mask_api_key`（≤8 位返回 `****`，否则首 4 + 星号 + 尾 4，参考旧版 `maskAPIKey`）。
- 前置：P1-1
- 验收：返回正确启用数量；Key 始终脱敏。
- PRD Story：53, 54, 55

### P4-8 Token 计数命令
- 所属层：Rust
- 文件：`src-tauri/src/modules/tokens.rs`、`src-tauri/src/commands/tokens.rs`
- 实现要点：`estimate_input_tokens`（system + messages 内容近似估算）；命令 `count_tokens(request)` 供 `/v1/messages/count_tokens` 与前端使用。参考旧版 `tokencount`。
- 前置：P0-2
- 验收：估算结果稳定、对长文本合理；单测覆盖 system 与 message。
- PRD Story：56, 57, 58

### P4-9 日志命令与实时推送
- 所属层：Rust
- 文件：`src-tauri/src/commands/logs.rs`、`src-tauri/src/modules/`（tracing layer）
- 实现要点：自定义 tracing layer 将日志行通过 `log-line` 事件推送前端；`set_log_level(level)` 动态调整；可选 `get_recent_logs` 返回环形缓冲最近 N 行。
- 前置：P0-6, P4-2
- 验收：前端能实时收到日志行；切换级别后输出变化。
- PRD Story：76, 77

## 参考材料（origin）

> 路径相对 `docs/origin/ccNexus/`

| 关注点 | 文件 | 关键符号 / 说明 |
|--------|------|-----------------|
| 端点 CRUD / 克隆 / 测试 | `internal/service/endpoint.go` | `extractBaseName`（克隆命名）、测试探测 |
| 配置读写 / 设置 | `internal/service/settings.go` | get/set 配置 |
| 安全配置白名单 | `internal/config/config.go` | `safeConfigKeys` |
| 模型列表 / 缓存 | `internal/proxy/models.go` | 30 分钟缓存、多端点拉取 |
| API Key 脱敏 | `internal/proxy/utils.go` | `maskAPIKey` 规则 |
| Token 估算 | `internal/tokencount/{estimator,image}.go` | system/messages 估算、图片 token |
| 日志级别 | `internal/logger/logger.go` | 动态级别 |
| 模型 API 文档 | `docs/models_api.md` | 模型列表接口形态 |

## 完成判据（里程碑 M3 之一）

- 配置/白名单、端点 CRUD/克隆/测试/排序、模型缓存命中、健康脱敏、Token 估算单测/手测通过；
- 改端口后代理在新端口监听，改日志级别即时生效，前端实时收到日志行。
