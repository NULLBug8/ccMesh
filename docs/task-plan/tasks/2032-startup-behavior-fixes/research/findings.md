# 启动行为需求.txt — 调研结论

需求 6 点：启动行为（自启动/静默/自动运行）+ Codex base_url 校验 + 点亮模型相关收尾 + 卡片 URL 放开。

## 1 自启动（跟随系统自启，默认关）
- 当前未接入任何开机自启能力。`Cargo.toml` 无 `tauri-plugin-autostart`；前端无 `@tauri-apps/plugin-autostart`。
- 方案：接入官方插件 `tauri-plugin-autostart`（`pnpm tauri add autostart` 会同时装 Rust crate + JS 绑定 + 写 capability）。
  - lib.rs 注册 `tauri_plugin_autostart::Builder::new().build()`（Windows 注册表 Run 项；macOS LaunchAgent；Linux）。
  - capability 加 `autostart:allow-enable / allow-disable / allow-is-enabled`。
  - 前端 JS API：`enable() / disable() / isEnabled()`。
- 状态真相源用插件自身（`isEnabled()`），设置页开关读 `isEnabled`、切换调 `enable/disable`。默认关 = 不主动 enable。

## 2 静默启动（后台启动，不展示窗口，默认关）
- 窗口已是 `tauri.conf.json` `visible:false`，由前端 `lib/boot.ts::revealMainWindow()` 在主题就绪后 show；
  `useThemeSync.ts` 首屏 reveal，`main.tsx` 3s 兜底 reveal，配置失败也 reveal。
- 方案：新增配置 `silentStart`（bool 默认 false）。前端 boot 时若 silentStart 为真则**跳过** reveal（含 3s 兜底与失败兜底），窗口留在托盘；用户经托盘左键/菜单「显示窗口」唤起（`tray.rs::show_main` 已具备）。
- 关键：silentStart 为真也要保证后续能正常显示（托盘已就绪），且不影响 single-instance 二次启动唤起（lib.rs 已 show+focus）。

## 3 自动运行（应用打开时自动运行=自动启动代理，默认开）
- 当前代理不自动启动：仅 Dashboard `ServiceCard` 手动启停、托盘 `tray-action` 启停。`commands/proxy.rs::start_proxy` 读 `read_port`(port 键) 起服务并存 `state.proxy`。
- 方案：新增配置 `autoRun`（bool 默认 true）。在 Rust `lib.rs` setup 末尾：若 autoRun 为真，读端口并 `start_server` 起代理、存入 `state.proxy`，emit `proxy-status-changed`。
  - 放后端 setup 而非前端，可覆盖静默启动（无 UI 交互）场景。
  - 注意 setup 是同步上下文，`start_server` 是 async：用 `tauri::async_runtime::spawn` 或 block_on；现有 `commands::proxy::start_proxy` 是 async command。setup 中用 `tauri::async_runtime::block_on` 起一次即可（端口读取走 config_repo）。

## 4 CodexWorkspace base_url / gateway 是否带 /v1（仅校验）
- `lib/toolConfig.ts::gatewayBaseUrl(port,"codex")` = `http://127.0.0.1:{port}/v1`（codex 末尾补 /v1，claude 不补）。
- `CodexWorkspace.tsx`：`gateway = gatewayBaseUrl(port,"codex")`，`defaultCodexToml(gateway)` 里 `base_url = "${gateway}"`，端点模式 baseUrl 也=gateway。
- 结论：**gateway 已含 /v1**，base_url 正确，无需改代码，仅记录验证结论。

## 5 点亮模型影响收尾 + CSS
- (a) 新建配置读端口：`ClaudeWorkspace/CodexWorkspace` 均 `port = cfgQ.data?.port ?? 3000`（["app-config"]），设置保存后 invalidate ["app-config"] 已刷新。已正确，仅验证。
- (b) 配置工作区模型下拉受点亮影响：两工作区 `advertised` 已用 `advertisedModels(ep)`（model 锁定→[model]；否则 activeModels 非空→activeModels；空→models，并入映射 from）。**已受点亮影响**，仅验证。
- (c) 模型映射按点亮：`ModelMappingDialog` 出站下拉用 `outboundModels(endpoint)` = model?[model]:models（**全量，未按 activeModels 过滤**）。需按需求改为按点亮过滤 —— 待澄清语义（见澄清 Q）。
- (d) index.css 亮色文字「白色太强」→ 建议 #52525B：亮色 `:root` 弱化文字 `--ink-mute:#71717a` / `--ink-disabled:#a1a1aa` 偏淡。需澄清具体目标 token/元素。
- (e) 未点亮模型「图标文字不发灰，正常显示」：`EndpointForm` 未点亮标签用 `variant="muted"` + `opacity-60`（亮色下偏淡）。可去 opacity-60 让其正常显示（muted 文字已是 #52525b）。需澄清范围（仅 EndpointForm？还是含卡片 hover/ModelList）。

## 6 卡片 URL 放开（支持 http 与 https）
- `EndpointCard.tsx::meta` 已是可点击按钮 `openUrl(endpoint.apiUrl)`（task 2031.4 完成）。
- 失败根因：`capabilities/default.json` 的 `opener:allow-open-url` 仅 allow `https://github.com/VkRainB/ccMesh/*`，其它域名被拦 → "Not allowed to open url ..."。
- 方案：放开 allow 为 http/https 全量。glob 语法（glob crate）：用 `http://**` 与 `https://**`（`**` 跨 `/`，覆盖含路径/端口的 URL）。可同时加 `opener:allow-default-urls`（官方默认放开 mailto/tel/http/https）。github 更新链接被 `https://**` 覆盖。
- 无法无头验证实际跳转，需本地点击端点卡片 URL 核对。

## 技术栈/验证命令
- 前端类型检查：`pnpm check:front`（tsc --noEmit）；测试 `pnpm test`（vitest run）。
- Rust：`pnpm check:rust`（cargo check）。整体 `pnpm check`。
- GUI/开机自启/托盘/真实跳转无法无头验证，给本地核对清单。
