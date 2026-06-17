# 启动行为 + 点亮收尾 + 卡片URL放开

## Goal
在设置中新增「启动行为」类别（自启动 / 静默启动 / 自动运行），并完成点亮模型的收尾影响、亮色文字可读性优化与端点卡片 URL 跳转放开，对齐 docs/task-plan/启动行为需求.txt。

## Requirements
1. 自启动：跟随系统开机自启，默认关。
2. 静默启动：后台启动、启动时不展示窗口、在后台（托盘）运行，默认关；每次启动均生效（含手动启动），经托盘唤起窗口。
3. 自动运行：应用打开时自动启动代理服务，默认开。
4. 校验 CodexWorkspace base_url=`${gateway}` 中 gateway 是否含 /v1 后缀（仅校验，预期已正确）。
5. 点亮模型收尾：
   - 新建配置（Claude/Codex 工作区）正确读取端口（仅校验）。
   - 配置工作区模型下拉受点亮模型影响（仅校验，已通过 advertisedModels 生效）。
   - 模型映射弹窗「出站模型」候选改为按点亮模型过滤。
   - 亮色模式弱化文字加深至约 #52525B 提升可读性；未点亮模型标签去掉灰化（半透明），正常显示。
6. 端点卡片 API URL 点击跳转放开 URL 限制，支持 http 与 https 任意地址。

## Acceptance Criteria
- [ ] 设置页出现「启动行为」类别，含三项开关，默认值：自启动关 / 静默启动关 / 自动运行开。
- [ ] 切换「自启动」实际写入/移除系统开机自启项；重开应用开关状态正确回显。
- [ ] 「静默启动」开启后，下次启动不弹窗、托盘常驻，点击托盘可显示窗口。
- [ ] 「自动运行」开启（默认）时应用启动后代理自动处于运行态；关闭时不自动启动。
- [ ] Codex 工作区 base_url 实际为 `http://127.0.0.1:{port}/v1`（含 /v1）。
- [ ] 模型映射弹窗出站候选只含点亮模型（未点亮任何项时回退全部）。
- [ ] 亮色模式下次要/弱化文字更易读；未点亮模型标签文字正常显示不发灰。
- [ ] 端点卡片点击 http/https URL 能在系统浏览器打开，不再报 "Not allowed to open url"。
- [ ] `pnpm check`（tsc + cargo check）与 `pnpm test` 通过。

## Definition of Done
- 全部验收项满足；无法无头验证的（开机自启注册、托盘静默、真实浏览器跳转）显式声明并给本地核对清单。
- progress.csv 子任务全部「完成」；按模块 scoped 提交。

## User Stories
- 作为用户，我希望应用能跟随系统开机自启并静默在后台运行，以便代理常驻无需每次手动开启。
- 作为用户，我希望应用打开即自动运行代理，以便立即可用。
- 作为用户，我希望模型映射与配置下拉只出现我点亮的模型，以便选择干净一致。
- 作为用户，我希望亮色模式文字清晰、未点亮模型也能看清名称，以便辨识。
- 作为用户，我希望点击端点卡片的 API 地址能在浏览器打开，以便快速访问。

## Implementation Decisions
- 自启动用官方插件 tauri-plugin-autostart，状态真相源用插件 isEnabled()；不单独落库（避免与系统态不一致）。
- 静默启动 / 自动运行落入 AppConfig（silentStart 默认 false，autoRun 默认 true），沿用 app_config 键值表与 get_config 解析。
- 静默启动在前端 boot 层实现（按配置跳过 revealMainWindow，含兜底）；自动运行在 Rust setup 实现（覆盖无 UI 的静默场景）。
- 模型映射出站候选：activeModels 非空→activeModels；空→全部（与 advertisedModels 基础集口径一致，但映射出站为真实模型，仍只在 models 集合内）。锁定 model 时为 [model]。
- 亮色文字：:root 加深 --ink-mute 与 --ink-disabled（约 #52525B 档），保留与 --ink-secondary 的层级差。
- 卡片 URL：放开 capability opener:allow-open-url 为 http/https 全量（glob `http://**`、`https://**`），保留对 github 更新链接的覆盖。
- silentStart / autoRun 纳入跨设备同步安全键白名单（SAFE_CONFIG_KEYS）。

## Testing Decisions
- 后端：config_repo 解析 silentStart/autoRun 默认值与读写往返单测。
- 前端：endpoint.ts 新增「点亮出站候选」纯函数单测（activeModels 过滤 / 空回退 / 锁定 model）。
- 其余 GUI/系统行为人工核对。

## Out of Scope
- 不改动代理转发逻辑、不动现有端口修复（2030.x）。
- 不为静默启动区分「开机自启 vs 手动」两种模式（按用户确认：统一每次生效）。
- 不重做托盘/窗口控制既有逻辑。

## Technical Notes
- 自启动 capability：autostart:allow-enable / allow-disable / allow-is-enabled。
- setup 自动运行：用 app.handle().clone() + tauri::async_runtime::spawn，await 前不持有 Mutex 守卫；复用 commands::proxy::build_status / PROXY_STATUS_EVENT（crate 内可见）。
- 无法无头验证：开机自启注册、静默托盘、真实浏览器跳转。
