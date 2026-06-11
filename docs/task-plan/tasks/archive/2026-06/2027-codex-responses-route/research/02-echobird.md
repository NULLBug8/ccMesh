# EchoBird 调研：codex 端点 / `/v1/responses` 路由 / Responses↔Chat 转换

> 调研对象：`E:\myCode\reference data\EchoBird`（Tauri，Rust 后端 + TS 前端）
> 目标：为 tauri-gateway 新增 codex 端点类型、`/v1/responses` 路由、Responses→Chat 转换、Responses 透传、代理 + 模型映射做对照移植参考。

---

## 0. 关键结论（先读这一条）

**EchoBird 这个检出（checkout）里，承载协议转换的 Rust 后端源码已被整体剥离，调研所需的核心转换代码不在磁盘上。**

- 整个仓库（排除 `node_modules`）只有 **3 个 `.rs` 文件**：
  - `src-tauri/src/main.rs`（6 行，仅调用 `echobird_lib::run()`）
  - `src-tauri/src/lib.rs`（135 行，**全部是 `include_str!` 打包安装 JSON + 托盘图标**，**没有一个 `#[tauri::command]`**，没有 `invoke_handler`，没有 `reqwest/hyper/axum`，没有 `apply_codex`、`proxy`、`config.toml`、`responses`、`completions` 等任何与转换相关的符号）
  - `src-tauri/src/build.rs`
- 前端通过 `invoke('apply_model_to_tool', ...)`、`invoke('scan_tools')` 等调用了大量 Tauri 命令，但**这些命令的 Rust 实现一个都不在源码树里**（`lib.rs` 没有注册它们）。说明发布到此目录的是「前端 UI + 极薄 Tauri 壳」，真正的后端 crate（`echobird_lib` 的真身）未包含。
- `tools/codex/README.md` 明确点名转换逻辑所在的文件路径（见下文第 3 节），但这些路径 `src-tauri/src/services/codex_proxy/*.rs` 在磁盘上**完全不存在**（`find` 验证：无 `services` 目录、无 `codex_proxy`、无 `protocol_converter`、无 `stream_handler`）。

**可移植性评估：转换逻辑的 Rust 实现无法直接照抄，因为它根本不在此检出中。** 但 README + 前端类型/UI 仍提供了**高价值的架构蓝图**（端口、配置文件形状、`wire_api`、三种模式语义、转换数据流、文件分层），足以指导我们自研。下文把所有能拿到的实证材料完整摘录，并把「README 描述但缺失」的部分明确标注，供后续决定是否需要去 EchoBird 的完整仓库 / 发布版二进制里补齐。

---

## 1. 端点 / 工具类型定义

EchoBird 没有「端点类型」枚举，它是按**工具（tool）**组织的，codex 是其中一个工具 id。相关类型在前端：

### `src/api/types.ts`

`DetectedTool`（每个工具的描述，`apiProtocol` 是协议数组）：

```ts
export interface DetectedTool {
  id: string;
  name: string;
  category: string;
  installed: boolean;
  detectedPath?: string;
  configPath?: string;
  activeModel?: string;
  website?: string;
  apiProtocol?: string[];   // 例如 ['openai'] / ['anthropic']
  iconBase64?: string;
  names?: Record<string, string>;
  startCommand?: string;
  launchFile?: string;
  command?: string;
  version?: string;
  noModelConfig?: boolean;
  launchUri?: string;
}
```

`ApplyModelInput`（下发给后端 `apply_model_to_tool` 的载荷，**codex 的两个开关在这里**）：

```ts
export interface ApplyModelInput {
  id: string;
  name: string;
  baseUrl: string;
  apiKey: string;
  model: string;
  protocol?: string;
  /**
   * Codex-only. When true, write the real upstream URL and API key
   * straight into ~/.codex/config.toml + auth.json so Codex talks to
   * the upstream directly. Bypasses our local protocol-bridging proxy.
   * Used for relay stations that already speak the Responses protocol.
   * Other tools ignore this field.
   */
  relayMode?: boolean;
  /**
   * Codex-only. When true, the local proxy stays in the path and still
   * rewrites the model id, but forwards the request to the upstream's
   * native `/responses` endpoint verbatim instead of translating it
   * down to Chat Completions. For third-party models that natively
   * support the Responses protocol but still need model-id rewriting
   * (so they can't use the proxy-bypassing relay mode). Mutually
   * exclusive with `relayMode`. Other tools ignore this field.
   */
  responsesPassthrough?: boolean;
}
```

> 移植要点：EchoBird 用 **两个布尔开关** 表达三种工作模式，正好对应本任务需求：
> 1. `relayMode=false && responsesPassthrough=false` → **走本地代理并把 Responses 翻译成 Chat Completions**（默认转换模式）
> 2. `relayMode=false && responsesPassthrough=true` → **走本地代理但 Responses→Responses 透传**（只改 model id，不降级翻译）
> 3. `relayMode=true` → **完全绕过本地代理**，直接把上游 URL/Key 写进 codex 配置（中转站本身已说 Responses）
> 两者互斥：打开 relay 会强制 passthrough=false（前端已做此约束，见第 5 节）。

工具 id 常量：codex 工具有两个变体 —— `codex`（CLI）与 `codexdesktop`（Desktop）。判断代码（`AppManagerProvider.tsx`）：

```ts
const isCodexApp = toolId === 'codex' || toolId === 'codexdesktop';
```

官方端点注册表 `src/data/officialEndpoints.ts`（用于「恢复官方」按钮，灵感来自 cc-switch）：

```ts
export interface OfficialEndpoint {
  name: string;
  baseUrl: string;            // OpenAI-protocol base URL
  anthropicUrl?: string;
  protocol: 'openai' | 'anthropic';
  modelId?: string;
}

export const OFFICIAL_ENDPOINTS: Record<string, OfficialEndpoint> = {
  // ...
  codex: {
    name: 'OpenAI Official',
    baseUrl: 'https://api.openai.com/v1',
    protocol: 'openai',
    modelId: 'gpt-4o',
  },
  codexdesktop: {
    name: 'OpenAI Official',
    baseUrl: 'https://api.openai.com/v1',
    protocol: 'openai',
    modelId: 'gpt-4o',
  },
  // ...
};
```

---

## 2. `/v1/responses` 路由：注册位置与 handler

**Rust 实现不在此检出中**（见第 0 节）。能拿到的实证仅来自 `tools/codex/README.md` 的架构描述，原文摘录：

```
                                                                   
   Codex CLI       Responses API    EchoBird Rust    Chat Completions   LLM Provider 
                 ─────────────►     (codex_proxy)   ───────────────►   (DeepSeek/    
   Codex Desktop                    127.0.0.1:53682                     Moonshot/etc)
                 ◄─────────────                     ◄───────────────                 
                  Responses SSE                       Chat SSE                       
```

> 1. **`apply_codex`**（在 `src-tauri/src/services/tool_config_manager.rs`）写出固定的 13 行 `~/.codex/config.toml`，让 Codex 看到 `base_url = "http://127.0.0.1:53682/v1"` 且 `wire_api = "responses"`。
> 2. **`spawn_proxy_task`** 在 Tauri 启动时绑定 `127.0.0.1:53682`，提供 `POST /v1/responses`。handler **每次请求都重新读取 `~/.echobird/codex.json`**，以便无需重启就能切换 model / API key / 上游 URL。
> 3. **Translation**：进来的 Responses 形状请求被转成 Chat Completions 请求，转发到用户的 provider，流式响应在出站时再翻译回 Responses 形状的 SSE。
> 4. **Smart spoof**：Codex 的 `gpt-5.4` model id 在转发前被改写成真实上游 model id，再在响应里映射回来，保证 Codex 的记账一致。

要点整理（用于本项目自研）：
- **监听地址固定**：`127.0.0.1:53682`，路由前缀 `/v1`，方法 `POST`，路径 `/v1/responses`。
- **配置热读取**：handler 每次请求读盘配置（`~/.echobird/codex.json`），避免重启。本项目可对应到自己的运行时配置存储。
- **codex 客户端侧约定**：写 `~/.codex/config.toml` 的 `wire_api = "responses"` + `base_url=.../v1`，codex 才会发 `/v1/responses`。

**缺失（需另行获取）**：`server.rs` 里 handler 函数签名、路由框架（README 未点名 axum/hyper）、503/401 错误处理细节。README 提到 503 文案 "No active model configured in EchoBird"（配置缺失时返回），可作为错误处理参考。

---

## 3. Responses API → Chat Completions 转换（最关键）

**核心转换源码不在此检出中。** `tools/codex/README.md` 的「Source map」表给出了官方文件分层（这是迄今最有价值的线索），原文摘录：

| Concern                       | Where it lives                                                |
| ----------------------------- | ------------------------------------------------------------- |
| HTTP server + handler         | `src-tauri/src/services/codex_proxy/server.rs`                |
| Responses → Chat translation  | `src-tauri/src/services/codex_proxy/protocol_converter.rs`    |
| Chat SSE → Responses SSE      | `src-tauri/src/services/codex_proxy/stream_handler.rs`        |
| Session history + reasoning   | `src-tauri/src/services/codex_proxy/session_store.rs`         |
| Multimodal content mapping    | `src-tauri/src/services/codex_proxy/content_mapper.rs`        |
| Relay / config.toml           | `src-tauri/src/services/codex_proxy/config_manager.rs`        |
| Onboarding skip               | `src-tauri/src/services/codex_proxy/onboarding_bypass.rs`     |
| Binary resolution             | `src-tauri/src/services/codex_proxy/codex_binary.rs`          |
| Spawn (CLI + Desktop)         | `src-tauri/src/services/process_manager.rs` (`start_codex_*`) |

验证：以上 `services/codex_proxy/*.rs` 文件在本检出磁盘上**全部不存在**（`find` 无任何匹配）。

README 关于转换的概念性描述（仅此而已，无字段级映射表，无代码原文）：
- 请求方向：**Responses-shape request → Chat Completions request**，转发到 provider。
- 响应方向：**streaming Chat SSE → Responses-shape SSE**。
- model id：转发前改写（spoof）成真实上游 id，响应里再映射回 codex 期望的 id。
- 会话/推理历史单独由 `session_store.rs` 维护，多模态内容由 `content_mapper.rs` 映射。
- 历史演进：v5.0 之前是 Node 版 `codex-launcher.cjs` 子进程跑代理；v5.0 起改为纯 Rust，跑在 Tauri 二进制内，SSE 转发缓冲更少。

**结论：请求体字段映射（input / instructions / messages / tools / tool_choice / stream / max_output_tokens）、响应体字段映射、SSE 事件逐条转换的具体实现，无法从此检出获得。** 这部分必须改为：
- 选项 A：从 EchoBird 的**完整源码仓库**或**已编译发布版**中提取 `protocol_converter.rs` / `stream_handler.rs`；
- 选项 B（推荐回退）：直接依据 OpenAI 官方 Responses API 与 Chat Completions API 规范自研映射（本项目已有 cc-switch transform 层可复用，见项目 MEMORY 中 `reference_cc_switch.md`）。

---

## 4. Responses API → Responses API 透传

**Rust 实现缺失。** 透传语义完整定义在前端类型注释里（已在第 1 节摘录 `responsesPassthrough` 字段）。关键语义：

- 触发判断：`responsesPassthrough === true` 且 `relayMode === false`（互斥，relay 优先）。
- 行为：**本地代理仍在链路中**，仍改写 model id，但把请求原样转发到上游的原生 `/responses` 端点，**不降级翻译成 Chat Completions**。
- 适用场景：第三方模型原生支持 Responses 协议、但仍需 model-id 改写（因而不能用「绕过代理」的 relay 模式）。

前端组合下发逻辑（`AppManagerProvider.tsx`，见第 5 节代码）已实现「三选一」分支，对应 README 的三条数据路径。

---

## 5. 代理开关 + 模型映射（前端侧，完整可用）

这是本检出里**唯一完整、可直接照抄思路**的部分。`src/pages/AppManager/AppManagerProvider.tsx` 的 `applyModelConfig` 把模式开关 + 模型映射组装后通过 IPC 下发：

```ts
const applyModelConfig = async (
  toolId: string,
  internalId: string,
  relayOverride?: boolean,
  passthroughOverride?: boolean
): Promise<true | string | false> => {
  const model = userModels.find((m) => m.internalId === internalId);
  if (!model) { console.error('Model not found:', internalId); return false; }

  const toolData = detectedTools.find((t) => t.id === toolId);
  const toolProtocols = toolData?.apiProtocol || ['openai'];

  const userSelectedProtocol =
    modelProtocolSelection[model.modelId || ''] || modelProtocolSelection[internalId];
  const selectedProtocol =
    userSelectedProtocol || (toolProtocols[0] === 'anthropic' ? 'anthropic' : 'openai');

  const useAnthropicUrl = selectedProtocol === 'anthropic' && model.anthropicUrl;
  const apiUrl = useAnthropicUrl ? model.anthropicUrl! : model.baseUrl;

  // Codex apps + Claude Desktop honor the relay-mode toggle.
  const isCodexApp = toolId === 'codex' || toolId === 'codexdesktop';
  const isClaudeDesktopApp = toolId === 'claudedesktop';
  const isRelayCapableApp = isCodexApp || isClaudeDesktopApp;
  const currentRelayMode = isClaudeDesktopApp ? claudeDesktopRelayMode : codexRelayMode;
  const effectiveRelay = relayOverride ?? currentRelayMode;
  // passthrough 与 relay 互斥；!effectiveRelay 保证不会两者同时为 true 下发
  const effectivePassthrough =
    isCodexApp && !effectiveRelay && (passthroughOverride ?? codexResponsesPassthrough);

  try {
    const result = await api.applyModelToTool(toolId, {
      id: model.internalId,
      name: model.name,
      baseUrl: apiUrl,                 // ← 上游地址（按协议在 baseUrl / anthropicUrl 间选）
      apiKey: model.apiKey,
      model: model.modelId || '',      // ← 模型映射：真实上游 model id
      protocol: selectedProtocol,
      ...(isRelayCapableApp ? { relayMode: effectiveRelay } : {}),
      ...(isCodexApp ? { responsesPassthrough: effectivePassthrough } : {}),
    });
    // ... 成功更新 activeModel；失败返回 message
  } catch (error) { /* ... */ }
};
```

互斥约束 setter（打开 relay 强制关 passthrough，并立即重写配置）：

```ts
const setCodexRelayMode = useCallback((v: boolean) => {
  setCodexRelayModeRaw(v);
  writeBool('echobird_codex_relay_mode', v);
  // 打开 API Router(relay) → 强制 Responses passthrough 关闭
  if (v) {
    setCodexResponsesPassthroughRaw(false);
    writeBool('echobird_codex_responses_passthrough', false);
  }
  const codexToolId = (['codex', 'codexdesktop'] as const).find((id) => !!toolModelConfig[id]);
  if (!codexToolId) return;
  const pendingInternalId = toolModelConfig[codexToolId];
  if (!pendingInternalId || isOfficialModelSentinel(pendingInternalId)) return;
  // 翻转后立即重新 apply（重写 ~/.codex/config.toml + auth.json）
  void applyModelConfig(codexToolId, pendingInternalId, v, v ? false : undefined)
    .then((result) => { if (result !== true) setApplyError(/* ... */); });
}, [toolModelConfig, t]);
```

**代理设置（HTTP proxy）**：本检出前端/Rust 均**未见**显式的「网络代理（proxy URL）」接入代码——README 提到的是「本地协议桥接代理（local proxy）」这层语义，而不是出站 HTTP 代理。出站到上游 provider 时是否走系统/自定义代理，其实现应在缺失的 `codex_proxy/server.rs` 转发段（reqwest client 构造处），此检出无从确认。**本项目的「代理开关」需自行实现，无法从 EchoBird 此检出移植。**

**模型映射**：实证为「前端把用户选择的真实 `model.modelId` 放进 `ApplyModelInput.model` 下发」，配合 README 描述的「转发前把 codex 的 `gpt-5.4` spoof 成真实上游 id、响应里再映射回来」。具体改写代码在缺失的 `protocol_converter.rs`。

---

## 6. 相关 IPC / 文件路径清单

前端 IPC 封装（`src/api/tauri.ts`）：

```ts
export async function applyModelToTool(toolId: string, modelInfo: ApplyModelInput)
  : Promise<{ success: boolean; message: string }> {
  return invoke('apply_model_to_tool', { toolId, modelInfo });
}
export async function restoreToolToOfficial(toolId: string)
  : Promise<{ success: boolean; message: string }> {
  return invoke('restore_tool_to_official', { toolId });
}
```

**本检出中真实存在、可参考的文件**：
- `E:\myCode\reference data\EchoBird\src\api\types.ts` — `DetectedTool` / `ApplyModelInput`（含 `relayMode`、`responsesPassthrough` 完整注释）/ `ModelConfig`
- `E:\myCode\reference data\EchoBird\src\api\tauri.ts` — `applyModelToTool` / `restoreToolToOfficial` IPC 封装
- `E:\myCode\reference data\EchoBird\src\pages\AppManager\AppManagerProvider.tsx` — 三模式开关组装 + 互斥逻辑 + 即时重写
- `E:\myCode\reference data\EchoBird\src\data\officialEndpoints.ts` — codex 官方端点注册表（cc-switch 风格）
- `E:\myCode\reference data\EchoBird\tools\codex\README.md` — **架构蓝图（端口、wire_api、数据流、文件分层 Source map）**，是缺失 Rust 代码的唯一描述来源
- `E:\myCode\reference data\EchoBird\src-tauri\src\lib.rs` / `main.rs` — 已剥离的薄壳，无转换逻辑

**README 点名但磁盘上不存在（移植所需的真正核心，需另行获取）**：
- `src-tauri/src/services/codex_proxy/server.rs` — HTTP server + `/v1/responses` handler
- `src-tauri/src/services/codex_proxy/protocol_converter.rs` — **Responses → Chat 转换（最关键）**
- `src-tauri/src/services/codex_proxy/stream_handler.rs` — **Chat SSE → Responses SSE 转换**
- `src-tauri/src/services/codex_proxy/session_store.rs` / `content_mapper.rs` / `config_manager.rs`
- `src-tauri/src/services/tool_config_manager.rs`（`apply_codex` 写 config.toml）
- `src-tauri/src/services/process_manager.rs`（`start_codex_*` / `spawn_proxy_task`）

---

## 7. 移植建议（结论）

1. **架构可照搬**：本地 `127.0.0.1:<port>/v1/responses` 监听 + 每请求热读配置 + 三模式（翻译 / Responses 透传 / relay 绕过）的设计成熟，直接采用。两个互斥布尔开关（`responsesPassthrough` / `relayMode`）的语义可原样复用。
2. **转换代码不可照抄**：字段级 Responses↔Chat 映射、SSE 事件转换的 Rust 实现**不在此检出**。落地两条路：(A) 从 EchoBird 完整仓库/发布二进制提取 `protocol_converter.rs`、`stream_handler.rs`；(B) 依据 OpenAI 官方规范自研，并复用本项目已有的 cc-switch transform 层（见 MEMORY `reference_cc_switch.md`）。鉴于核心缺失，**推荐以 (B) 为主、(A) 为补充对照**。
3. **代理（出站 HTTP proxy）开关需自研**：EchoBird 此检出无相关实证。
4. **模型映射**：沿用「下发真实 model id + 代理层转发前 spoof / 响应回映」两段式即可。
