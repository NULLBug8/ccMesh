# PRD（第四轮）：代理功能体系化（直连 / 全局代理 / 专用代理）

> 状态：ready-for-agent
> 来源：`需求4.txt` + `设置代理截图.png` + 上一轮代理链路分析、需求对齐
> 适用范围：tauri-gateway（r-app，后端 Rust/Tauri + axum；前端 React 19）
> 关联：[PRD.md](./PRD.md) · [PRD-2.md](./PRD-2.md) · [PRD-3.md](./PRD-3.md) · [TASKS.md](./TASKS.md) · 子文档 [09-proxy-redesign.md](./09-proxy-redesign.md)

## Problem Statement

当前代理能力零散且有坑：只有「端点级 `use_proxy` + 全局 `proxyUrl`」一条路径，且当 `use_proxy=true` 但代理地址无效/为空时**静默回退直连**（无日志）；「获取模型」(`commands/models.rs`) 写死 `.no_proxy()` 永远直连，与端点代理意图不一致；应用更新不经代理；设置里只有一个「代理地址」输入框，缺少「启用代理 / 代理更新」开关与「测试」入口（见截图）。用户期望一个清晰的三档模型：直连 / 全局代理 / 专用代理，并有明确优先级。

## Solution

把代理统一为「是否走代理」的分层决策 + 设置页三件套（启用代理开关 / 代理服务器地址+测试 / 代理更新开关）。

**优先级（高→低）**：端点配置代理（`use_proxy`）> 设置「启用代理」总开关（`proxyEnabled`）> 「代理更新」专用开关（`proxyForUpdate`，UI 上受「启用代理」门控）。

**三档语义**：
- **直连**：未填代理地址（`proxyUrl` 空）→ 一切直连。
- **全局代理**：填了地址且「启用代理」开 → 请求转发 + 获取模型 经代理出网（端点未单独开 `use_proxy` 时按此总开关）。
- **专用代理**：针对特定功能单独控制。本期一个：**代理更新**——开启则应用更新检查/下载经同一代理地址出网。
- **端点级覆盖**：端点 `use_proxy` 开（且有地址）→ 该端点的转发 + 获取模型**始终走代理**，不受总开关影响（最高优先级）。

**统一判定**：
- 转发 + 获取模型：`走代理 = (端点 use_proxy || proxyEnabled) && proxyUrl≠""`。
- 应用更新：`走代理 = proxyForUpdate && proxyUrl≠""`（前端 `proxyForUpdate` 仅在 `proxyEnabled` 开时可切换）。
- 本地处理/读库路由（`/v1/models` 读库、`/health`、`/stats`、`/v1/messages/count_tokens`）**永不走代理**（无歧义）。

## User Stories

1. 作为使用者，未填代理地址时一切直连，无需额外设置。
2. 作为使用者，填了代理地址并打开「启用代理」后，网关转发请求经代理出网。
3. 作为使用者，打开「启用代理」后，「获取模型」也经代理出网（不再写死直连）。
4. 作为使用者，我希望某个端点单独打开「经代理出网」开关后，该端点的转发与获取模型始终走代理，即使全局「启用代理」是关的（端点级最高优先级）。
5. 作为使用者，端点级代理开关只有在填了代理地址时才真正生效（无地址则直连）。
6. 作为使用者，我希望「代理更新」开关控制应用更新是否经代理检查/下载，读取同一代理地址。
7. 作为使用者，我希望「代理更新」开关在「启用代理」关闭时被禁用并视为关（上一级没开就禁用）。
8. 作为使用者，我希望代理服务器输入框旁有「测试」按钮，点按经当前代理地址做一次连通性检测，返回成功与延迟。
9. 作为使用者，我希望代理地址支持 `host:port`（按 http 处理）与带 scheme 的 `http://...`，并有示例占位提示。
10. 作为使用者，我希望本地处理/暴露给本地的路由（模型列表读库、健康、统计、token 计数）永不走代理。
11. 作为使用者，当我开了代理但地址无效/为空导致回退直连时，希望后端有日志告警（不再完全静默），便于排查。
12. 作为使用者，改动代理地址或「启用代理」后希望立即生效（运行中的代理自动以新配置重启）。
13. 作为开发者，我希望"按是否走代理构建 HTTP client"的逻辑收敛为一个公共函数，转发/测试/获取模型/更新复用，避免漂移。

## Implementation Decisions

### 配置（新增键）
- `proxyEnabled`（bool，默认 false）：全局「启用代理」总开关。
- `proxyForUpdate`（bool，默认 false）：「代理更新」专用开关。
- 沿用 `proxyUrl`（既有）。三者均为设备相关，**不进** `SAFE_CONFIG_KEYS`（不跨设备同步）。
- `AppConfig` 增 `proxy_enabled`/`proxy_for_update`；`config_repo::get_config` 解析；前端 `AppConfig` 同步。
- `set_config` 重启触发：现有 `proxyUrl/openaiUa/claudeCliUa` 基础上**增加 `proxyEnabled`**（影响运行期转发决策）；`proxyForUpdate` 不需重启（更新命令运行时读配置）。

### 公共 client 构建（DRY）
- 新增 `modules/proxy/client.rs::build_client(want_proxy, proxy_url, timeout) -> reqwest::Client`：`want_proxy && 地址有效` → `.proxy()`；否则 `.no_proxy()`（显式禁用系统代理）；地址无效则 warn 日志后回落直连。供 test_endpoint / models / test_proxy 复用；转发热路径的 client（含连接池参数）仍在 `server.rs` 构建，但决策逻辑一致。

### 转发（server.rs / forward.rs）
- `ProxyState` 增 `proxy_enabled: bool`（start_proxy 时读配置）。`proxy_client` 仍在 `proxyUrl` 非空时构建。
- `send_upstream`：`want = ep.use_proxy || st.proxy_enabled`；`want && proxy_client=None` 时 warn（修静默回退）。

### 获取模型（commands/models.rs）
- 去掉写死 `.no_proxy()`。`get_models` 按每个端点 `use_proxy || proxyEnabled` 用 `build_client` 构建（读一次 proxyEnabled/proxyUrl）；`fetch_endpoint_models` 增 `use_proxy` 入参 + 读全局配置，决策 `use_proxy || proxyEnabled`。

### 测试连通性（commands）
- `test_endpoint`：代理决策由 `use_proxy && 地址` 改为 `(use_proxy || proxyEnabled) && 地址`，复用 `build_client`。
- 新增 `test_proxy(url) -> TestResult`：用该地址构建代理 client，GET 一个轻量连通性 URL（generate_204 类）短超时，返回 success/latencyMs/message。

### 应用更新（commands/update.rs）
- `check_for_updates`/`download_and_install` 增 `state: State<AppState>`（前端调用不变）；`proxyForUpdate && 地址非空` 时用 `app.updater_builder().proxy(url).build()`，否则默认 updater。

### 前端设置页（按截图）
- 新增「代理」区块：①「启用代理」Switch（proxyEnabled）；②「代理服务器」Input（proxyUrl）+「测试」按钮 + 示例提示；③「代理更新」Switch（proxyForUpdate），`proxyEnabled` 关时 disabled 且呈关。
- `config.ts` `AppConfig` 增 `proxyEnabled`/`proxyForUpdate`；端点表单「刷新模型」把 `use_proxy` 传给 `fetch_endpoint_models`。

## Testing Decisions

- **公共 client 构建（后端纯逻辑）**：`build_client` —— want_proxy+有效地址 → 带 proxy；空/无效/!want → 直连（不 panic）。可断言"是否设置了 proxy"较难直接读 reqwest，故抽一个**纯决策函数** `should_use_proxy(use_proxy, proxy_enabled, proxy_url) -> bool` 并单测（覆盖优先级真值表），client 构建薄封装其上。
- **更新代理决策（后端）**：`should_proxy_update(proxy_for_update, proxy_url) -> bool` 纯函数单测。
- 沿用 `rotation.rs`/`circuit_breaker.rs` 的纯逻辑单测风格；不为 reqwest 真实出网写网络依赖测试。
- **前端（vitest）**：设置页「代理更新」在「启用代理」关时 disabled；保存回写正确的扁平键（proxyEnabled/proxyForUpdate/proxyUrl）。

## Out of Scope

- SOCKS 代理（reqwest 未启用 `socks` 特性）：本期仅 HTTP/HTTPS 代理；如需 SOCKS 另起（加 feature）。
- 每端点独立"强制直连"语义：`use_proxy` 为"opt-in 走代理"，与全局是 OR 关系，不提供端点级强制直连。
- 多个专用代理通道：本期专用代理仅「代理更新」一个。
- 代理鉴权（用户名/密码内嵌 URL 以外的形式）。

## Further Notes

- 决策真值表（地址非空时）：转发/获取模型 `proxy = use_proxy || proxyEnabled`；更新 `proxy = proxyForUpdate`。地址为空 → 全部直连。
- 修复上一轮分析发现的"静默回退直连"：`want_proxy` 但无可用代理 client 时打 warn。
- `proxyForUpdate` 后端独立判断（`proxyForUpdate && 地址非空`），前端保证其仅在 `proxyEnabled` 开时可切换（门控为 UI 行为）。
