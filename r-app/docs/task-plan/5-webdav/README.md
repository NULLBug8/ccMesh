# 阶段 5：WebDAV 同步

> 任务总线：[../../TASKS.md](../../TASKS.md) ｜ 规划索引：[../README.md](../README.md)
> 起点参考：旧版 ccNexus 源码 `docs/origin/ccNexus/`（Go + Wails，仅参考不复用）

## 阶段目标

封装 WebDAV 客户端，实现连接测试、数据库备份/恢复（含元数据）、备份列表/删除、配置与统计同步（上行前按白名单过滤设备特定项），并提供前端同步页面。交付 **里程碑 M4 同步与体验** 的同步部分。

## 前置依赖

- 阶段 0（P0-2 错误、P0-4 库 / 数据目录、P0-5 设备 ID）；
- 阶段 4（P4-1 `safe_config_keys`）。

## 任务清单

### P5-1 WebDAV 客户端封装
- 所属层：Rust
- 文件：`src-tauri/src/models/webdav.rs`、`src-tauri/src/modules/webdav/client.rs`、`mod.rs`
- 实现要点：用 `reqwest_dav` 封装 `connect/upload/download/list/delete`；模型 `WebDavConfig`、`BackupFile`、`BackupMeta`、`TestResult`。
- 前置：P0-2
- 验收：对接测试服务器可上传/下载/列举/删除。
- PRD Story：31, 35, 36

### P5-2 连接测试命令
- 所属层：Rust
- 文件：`src-tauri/src/commands/webdav.rs`
- 实现要点：`test_webdav_connection(config)`：探测可达性与认证，返回 `TestResult`。
- 前置：P5-1
- 验收：正确凭证返回成功，错误返回明确失败原因。
- PRD Story：37

### P5-3 备份/恢复与元数据
- 所属层：Rust
- 文件：`src-tauri/src/modules/webdav/sync.rs`、`src-tauri/src/commands/webdav.rs`
- 实现要点：`backup_database`（读 DB 文件 + 写元数据 JSON：备份时间、版本）；`restore_database`（下载覆盖，恢复后重连池）；`list_backups`、`delete_backups`。参考旧版 `sync.go`。
- 前置：P5-1, P0-4
- 验收：备份后云端含 DB 与 meta；恢复后数据可读。
- PRD Story：33, 34, 35, 36, 38

### P5-4 配置/统计同步与设备过滤
- 所属层：Rust
- 文件：`src-tauri/src/modules/webdav/sync.rs`、`src-tauri/src/commands/webdav.rs`
- 实现要点：`sync_config`、`sync_stats`：上行/下行配置与统计；上行前用 `safe_config_keys` 过滤设备特定项（device_id、窗口行为等），避免多设备冲突。
- 前置：P5-3, P4-1, P0-5
- 验收：同步后设备特定项不被覆盖；统计按 device_id 合并不丢失。
- PRD Story：31, 32, 39

### P5-5 WebDAV 前端
- 所属层：React
- 文件：`src/pages/Sync/index.tsx`、`src/pages/Sync/_components/*`（`WebdavForm`/`BackupList`/`ConnTest`）、`src/hooks/useWebdav.ts`、`src/services/modules/webdav.ts`(webdavApi)
- 实现要点：WebdavForm 配置 URL/账号/密码/路径（Input/Label，密码字段脱敏显示）；ConnTest 按钮触发测试并 toast；BackupList 列出备份（Card/列表）支持恢复与删除（Dialog 确认）；备份/同步操作期间用 Badge/loading 状态。
- 前置：P5-2, P5-3, P5-4, P0-8
- 验收：可配置、测试、备份、恢复、列出、删除全流程可用。
- PRD Story：31-39

## 参考材料（origin）

> 路径相对 `docs/origin/ccNexus/`

| 关注点 | 文件 | 说明 |
|--------|------|------|
| WebDAV 客户端 | `internal/webdav/client.go` | connect/upload/download/list/delete |
| 同步 / 备份 / 恢复 / 设备过滤 | `internal/webdav/sync.go` | 同步流程、过滤逻辑 |
| WebDAV 类型 | `internal/webdav/types.go` | 配置/备份元数据结构 |
| 服务封装 | `internal/service/webdav.go` | 对上层暴露的同步服务 |
| 备份元数据 / 本地备份 | `internal/service/{backup,backup_helpers,backup_local,backup_types}.go` | 备份时间/版本元数据 |
| 安全配置过滤 | `internal/config/config.go` | `safeConfigKeys`（与 P4-1 共用） |
| 前端同步交互 | `cmd/desktop/frontend/src/modules/webdav.js` | 表单/备份列表 UX |

## 完成判据（里程碑 M4 之一）

- 连接测试、备份/恢复（含 meta）、列表/删除、配置/统计同步全链路可用；
- 同步上行按白名单过滤设备特定项，多设备不互相覆盖（详见 P10-5 集成测试）。

> 备注：正式 WebDAV 测试环境暂未提供，开发期以本地 stub（dufs / `rclone serve webdav`）占位。
