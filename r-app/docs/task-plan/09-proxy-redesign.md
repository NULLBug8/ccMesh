# 09 — WP10 代理功能体系化（直连/全局/专用）

> 关联：[TASKS.md](./TASKS.md) · [PRD-4.md](./PRD-4.md)
> 所属层：后端（Rust/Tauri/axum）+ 前端（设置页）
> 原始需求：[需求4.txt](./需求4.txt) · 截图 `设置代理截图.png`

## 目标

统一代理决策为分层模型：端点 `use_proxy` > 全局 `proxyEnabled` > 专用 `proxyForUpdate`。转发 + 获取模型按 `(use_proxy || proxyEnabled) && 地址非空` 走代理；应用更新按 `proxyForUpdate && 地址非空`；本地路由永不走代理。设置页按截图提供 启用代理 / 代理服务器+测试 / 代理更新 三件套。修上一轮"静默回退直连"。

## 关键文件/落点

- 配置：`models/config.rs`（+proxy_enabled/proxy_for_update）、`modules/storage/config_repo.rs`（get_config 解析）、`commands/config.rs`（set_config 重启触发 +proxyEnabled）。
- 公共逻辑：新增 `modules/proxy/client.rs`（`should_use_proxy` 纯函数 + `build_client` + `should_proxy_update`）+ 单测；`modules/proxy/mod.rs` 注册。
- 转发：`modules/proxy/forward.rs`（ProxyState +proxy_enabled；send_upstream 决策 `use_proxy||proxy_enabled` + 静默回退 warn）、`modules/proxy/server.rs`（start_proxy 读 proxy_enabled）。
- 获取模型：`commands/models.rs`（去 `.no_proxy()`，按决策 build_client；fetch_endpoint_models +use_proxy 入参 + State）。
- 测试：`commands/endpoint.rs::test_endpoint`（决策改 `use_proxy||proxy_enabled`）、新增 `test_proxy` 命令；`lib.rs` 注册。
- 更新：`commands/update.rs`（check/download +State；proxyForUpdate 时 updater_builder().proxy()）。
- 前端：`services/modules/config.ts`（AppConfig +proxyEnabled/proxyForUpdate）、`services/modules/health.ts` 或 proxy 服务（test_proxy 封装）、`pages/Settings/index.tsx`（代理区块三件套 + 门控）、`services/modules/endpoint.ts` + `EndpointForm`（刷新模型传 use_proxy）。

## 任务拆解

- **10.1** 公共代理决策 + client 构建：`modules/proxy/client.rs`（`should_use_proxy(use_proxy, proxy_enabled, url)`、`should_proxy_update(for_update, url)`、`build_client(want, url, timeout)`）+ 真值表单测；mod 注册。
- **10.2** 配置：AppConfig/config_repo/前端 config.ts 增 proxyEnabled/proxyForUpdate；set_config 重启触发加 proxyEnabled。
- **10.3** 转发集成：ProxyState +proxy_enabled；server.rs 读取；send_upstream 决策 + 静默回退 warn。
- **10.4** 获取模型走代理：models.rs 两处去 no_proxy，按决策构建；fetch_endpoint_models +use_proxy 入参 + 读全局；前端 endpoint.ts/EndpointForm 传 use_proxy。
- **10.5** 测试：test_endpoint 决策更新；新增 test_proxy 命令（连通性检测）+ 注册 + 前端封装。
- **10.6** 更新走代理：update.rs check/download 接 State，proxyForUpdate 时经代理 updater。
- **10.7** 设置页 UI：启用代理 Switch + 代理服务器 Input+测试 + 代理更新 Switch（proxyEnabled 关时禁用）+ 单测。

构建顺序：10.1/10.2（地基）→ 10.3/10.4/10.5/10.6（后端各路径）→ 10.7（前端 UI）。

## 数据契约

```
AppConfig 增： proxyEnabled: boolean, proxyForUpdate: boolean
test_proxy(url: string) -> { success, status, latencyMs, message }   // 复用 TestResult 形态
fetch_endpoint_models(apiUrl, apiKey, transformer, useProxy) -> string[]
```
决策真值表（地址非空）：转发/获取模型 `proxy = use_proxy || proxyEnabled`；更新 `proxy = proxyForUpdate`。地址空 → 全直连。

## 验收标准

- 未填地址：转发/获取模型/更新全直连。
- 填地址 + 启用代理：转发与获取模型走代理；关闭则直连（端点未单独开时）。
- 端点 use_proxy 开：该端点转发+获取模型始终走代理（即使全局关）。
- 代理更新开（启用代理开）：更新检查/下载走代理；启用代理关时该开关禁用且视为关。
- 测试按钮：经当前地址连通性检测返回成功/延迟；地址无效给出失败信息。
- want_proxy 但无可用代理 client 时后端 warn（非静默）。
- 改 proxyUrl/proxyEnabled 后运行中的代理自动重启生效。
- 本地路由（/v1/models 读库、/health、/stats、count_tokens）不走代理。

## 测试点

- 后端 `client.rs`：`should_use_proxy` 真值表（use_proxy/proxyEnabled/地址空 组合）；`should_proxy_update`（for_update + 地址）。
- 前端（vitest）：设置页代理更新在启用代理关时 disabled；保存写出正确扁平键。

## 提交策略（WP10）

- `10.1/10.2 决策+配置` 一组；`10.3 转发` 一组；`10.4 获取模型` 一组；`10.5 测试(含 test_proxy)` 一组；`10.6 更新` 一组；`10.7 设置页 UI` 一组。
