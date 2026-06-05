# 阶段 6：托盘、主题与多语言

> 任务总线：[../../TASKS.md](../../TASKS.md) ｜ 规划索引：[../README.md](../README.md)
> 起点参考：旧版 ccNexus 源码 `docs/origin/ccNexus/`（Go + Wails，仅参考不复用）

## 阶段目标

实现系统托盘（图标/菜单/窗口行为，文案随语言重建）、窗口关闭行为（直接关闭/最小化托盘/每次询问）、主题选择持久化与定时自动切换、多语言（zh/en）全界面覆盖。交付 **里程碑 M4 同步与体验** 的体验部分。

> 注意：明暗双主题 token、`ThemeToggle`、`next-themes` 切换已在设计基线（`DESIGN.md` / `LAYOUT.md`）落地，本阶段主题任务**不重建调色板**，只做持久化与定时切换。

## 前置依赖

- 阶段 0（P0-6 setup、P0-7 i18n 骨架、P0-8 布局/ThemeToggle）；
- 阶段 4（P4-1/P4-2 配置读写：主题/语言/窗口行为）。

## 任务清单

### P6-1 系统托盘
- 所属层：Rust
- 文件：`src-tauri/src/modules/tray.rs`，`lib.rs` setup 接入，`src-tauri/icons/`
- 实现要点：用 Tauri tray API 创建托盘图标与菜单（显示窗口/启停代理/退出）；双击/左键显示主窗口；右键展开菜单；菜单文案随语言重建。
- 前置：P0-6, P4-2
- 验收：托盘图标出现，双击显示窗口，菜单项可操作。
- PRD Story：40, 41, 42, 43

### P6-2 窗口关闭行为
- 所属层：Rust + React
- 文件：`src-tauri/src/commands/window.rs`、`lib.rs`（窗口 close 事件拦截）、`src/pages/Settings/index.tsx`
- 实现要点：监听窗口 `CloseRequested`，依据 `close_window_behavior`（直接关闭/最小化到托盘/每次询问）处理；「每次询问」时前端弹 Dialog 选择。设置页提供该选项。
- 前置：P6-1, P4-2
- 验收：三种行为均符合预期；询问模式记忆可选。
- PRD Story：44

### P6-3 多主题与 token 定义
- 所属层：React
- 文件：`src/index.css`、`src/components/common/ThemeToggle.tsx`、`src/stores/modules/settings.ts`
- 实现要点：明暗 token、`ThemeToggle` 与 `next-themes` 切换已在设计基线落地（见 TASKS.md §二 设计系统基线），无需重建；本任务聚焦：将主题选择持久化到后端 `app_config` 并与 `next-themes` 双向同步，启动时按持久化值恢复；如需扩展更多主题再在 `index.css` 追加调色板。
- 前置：P4-2, P0-8
- 验收：切换主题即时生效并重启后保持。
- PRD Story：45

### P6-4 定时自动切换主题
- 所属层：React
- 文件：`src/components/common/ThemeToggle.tsx` 或 `src/hooks/useAutoTheme.ts`、`src/pages/Settings/index.tsx`
- 实现要点：`theme_auto` 开启时，按 `auto_light_theme`（默认 7:00-19:00）与 `auto_dark_theme`（19:00-7:00）配置定时切换；设置页可配置时间区间；用定时器在跨界时刻切换 next-themes。
- 前置：P6-3
- 验收：到达配置时间自动切换；可自定义时间区间。
- PRD Story：46, 47, 48

### P6-5 多语言（i18n）
- 所属层：React
- 文件：`src/lib/i18n.ts`、`src/locales/zh.ts`、`src/locales/en.ts`、`src/components/common/LangToggle.tsx`
- 实现要点：完善 `t(key)` 与中英文资源，覆盖全部界面文案（导航、表单、按钮、提示、错误、托盘需要的文案经命令回传）；语言切换持久化（localStorage + 后端 config），并触发托盘重建。
- 前置：P0-7, P6-1
- 验收：全界面切换中英文无遗漏；重启保持；托盘随之更新。
- PRD Story：59, 60, 61, 43

## 参考材料（origin）

> 路径相对 `docs/origin/ccNexus/`

| 关注点 | 文件 | 说明 |
|--------|------|------|
| 托盘（Windows/macOS/other） | `internal/tray/{tray_windows,tray_darwin,tray_other,icon}.go` | 菜单项、双击显示、图标 |
| 窗口行为 / 托盘联动 | `cmd/desktop/app.go` | 关闭行为、最小化托盘 |
| 主题/语言/窗口配置 | `internal/service/settings.go` | 配置项与默认值 |
| i18n 机制与文案 | `cmd/desktop/frontend/src/i18n/{index,zh-CN,en}.js` | key 划分、覆盖范围 |
| 主题样式（仅参考） | `cmd/desktop/frontend/src/themes/*.css` | 本项目用 Dark Stripe，不照搬 |
| 关闭行为/语言文案 | `cmd/server/webui/ui/js/i18n/*` | 另一份 i18n 参考 |

## 完成判据（里程碑 M4 之一）

- 托盘图标/菜单可用、双击显示窗口、菜单随语言重建；
- 三种窗口关闭行为正确；
- 主题选择持久化 + 定时自动切换可用；
- zh/en 全界面切换无遗漏并持久化。
