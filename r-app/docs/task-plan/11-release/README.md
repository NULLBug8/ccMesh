# 阶段 11：打包与发布

> 任务总线：[../../TASKS.md](../../TASKS.md) ｜ 规划索引：[../README.md](../README.md)
> 起点参考：旧版 ccNexus 源码 `docs/origin/ccNexus/`（Go + Wails，仅参考不复用）

## 阶段目标

完善应用元信息与图标、收敛 CSP 与 capabilities 权限、产出安装包与更新构件。交付 **里程碑 M6 发布就绪**。

## 前置依赖

- 阶段 9（P9-1 更新插件/构件）；
- 全部功能阶段完成（0–10）。

## 任务清单

### P11-1 应用元信息与图标
- 所属层：Rust
- 文件：`src-tauri/tauri.conf.json`、`src-tauri/icons/`、`Cargo.toml`
- 实现要点：更新 `productName` 为 ccNexus、设置正式 `identifier`、窗口尺寸/最小尺寸/居中；用 `pnpm tauri icon <1024png>` 生成全套图标。
- 前置：P0-6
- 验收：窗口标题/图标正确。
- PRD Story：发布

### P11-2 CSP 与 capabilities 收敛
- 所属层：Rust
- 文件：`src-tauri/tauri.conf.json`、`src-tauri/capabilities/default.json`
- 实现要点：生产环境设置合理 CSP；capabilities 仅授予所需权限（opener、updater、process、tray、以及无边框窗口控制 `core:window` 的 minimize/maximize/unmaximize/close/start-dragging 等）。
- 前置：各阶段插件接入完成
- 验收：应用功能不受限且无多余权限。
- PRD Story：安全、发布

### P11-3 构建与安装包
- 所属层：Rust
- 文件：构建脚本/说明
- 实现要点：`pnpm tauri build` 产出 Windows（nsis/msi）安装包；配置签名环境变量产出更新构件；记录跨平台构建注意事项。
- 前置：P9-1, P11-1, P11-2
- 验收：生成可安装包并能正常启动运行。
- PRD Story：发布、71-75

## 参考材料（origin）

> 路径相对 `docs/origin/ccNexus/`

| 关注点 | 文件 | 说明 |
|--------|------|------|
| 旧版打包配置 | `cmd/desktop/wails.json` | 产品名/标识/窗口（对标 `tauri.conf.json`） |
| CI 构建 / 发布流程 | `.github/workflows/build.yml` | 多平台产物、签名、发布步骤 |
| 开发 / 构建文档 | `docs/development.md`、`docs/development_en.md` | 构建注意事项 |
| 服务端打包（仅参考） | `cmd/server/{Dockerfile,docker-compose.yml}` | 本项目桌面优先，不在范围 |
| 应用图标源 | `cmd/desktop/build/appicon.png`、`docs/images/ccNexus.svg` | 图标素材参考 |

## 完成判据（里程碑 M6）

- `pnpm tauri build` 产出 Windows 安装包并能正常启动；
- CSP / capabilities 收敛到最小必要权限；
- 更新构件随构建产出（endpoints/pubkey 待分发渠道确定后填入）。
