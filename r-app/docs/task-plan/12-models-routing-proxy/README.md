# 阶段 12：端点模型管理 / 多格式入站 / 代理 / UA

> 本方案来源于 [`../Question.md`](../Question.md) 的 6 条问题，经与需求方逐条讨论必要性后定稿。
> 任务编号规则 `P12-{序号}`，进度跟踪见同目录 [`progress.csv`](./progress.csv)。
> 全文相对路径以 `r-app` 项目根为基准。定稿日期：2026-06-06。

---

## 一、背景与问题来源

当前网关（ccNexus 重构版，Tauri 2 + axum）的代理核心存在一个隐含架构假设：**入站请求一律按 Claude Messages 格式处理**（`forward.rs::handle_proxy` 把请求体当 Claude，按端点 transformer 决定是否转成 OpenAI）。同时端点数据模型是「一端点一模型」（`endpoint.model` 单值），`/v1/models` HTTP 路由是空占位，无代理支持，UA 为隐式透传。

Question.md 的 6 条问题本质围绕同一主题：**让网关成为「以 Claude 入站为主、兼容 OpenAI 入站」的多端点轮换网关，并补齐端点的一对多模型管理、代理与 UA 控制**。

| # | 原始问题 | 必要性结论 |
|---|----------|-----------|
| 1 | `/v1/models` 按启用端点返回模型列表 | 必要，低成本（聚合逻辑已存在，路由占位） |
| 2 | 各端点展示各自模型（grid 卡片 + 悬停浮层） | 必要（按端点分组）；浮层需限高可滚动 |
| 3 | 添加端点支持 自定义模型 + 刷新拉取 + 选中才展示 | 必要，改动最大（动 schema） |
| 4 | UA 传递控制（伪装 OpenAI / claude-cli UA） | 防护性需要，按上游格式选用 |
| 5 | 端点级「启用代理」开关 + 全局代理地址 | 全局代理必要；端点级开关已确认要做 |
| 6 | gpt 端点 `/v1/chat/completions` 入站 | 真问题，必修（缺入站格式识别，非缺路由） |

---

## 二、已确认设计决策

讨论中已与需求方逐条拍板，作为本方案的不可动摇前提：

- **A. 入站格式定位**：Claude 入站为主（`/v1/messages` 现状不变）；OpenAI 客户端发 `/v1/chat/completions` 时按 OpenAI 入站处理。**最小版**：OpenAI 入站只透传到 OpenAI 端点，不做 OpenAI↔Claude 交叉转换。
- **B. 端点—模型关系**：端点从「一对一」改为「一对多」。
  - 新增 `models[]`：该端点对外暴露/已选的模型清单（**聚合型**端点用，如 OpenRouter 类）。
  - `model`（保留单值）语义重定义为「**可选锁定模型**」：填了 = 该端点锁死此模型并覆盖客户端请求的 model（**专用型**端点）；留空 = 透传客户端 model。
- **C. 代理粒度**：全局代理地址（`AppConfig.proxyUrl`）+ 端点级开关（`endpoint.useProxy`）。转发层维护「直连 / 代理」两个 reqwest client，按端点 `useProxy` 选用。
- **D. UA**：设置「高级设置」中两个可填字段 `openaiUa` / `claudeCliUa`。填了 = 覆盖；留空 = 透传客户端 UA（并补保护：客户端未带 UA 时不泄露 reqwest 默认值）。**选用规则**：转发到 OpenAI 端点用 `openaiUa`，转发到 Claude 端点用 `claudeCliUa`。
- **E. 浮层**：grid 卡片悬停在可用性指示上弹浮层显示该端点模型，浮层**限高 + 可滚动**。

---

## 三、核心设计澄清（贯穿全阶段）

### 3.1 两条独立的「模型」路径，勿混淆

| 路径 | 数据来源 | 用途 | 触发时机 |
|------|---------|------|---------|
| **配置态模型清单** | 读 DB `endpoints.models[]` / `model` | `/v1/models` 对外公布 + 前端按端点分组展示 + 卡片浮层 | 实时读库，极快，不请求上游 |
| **候选模型拉取** | 实时请求上游 `/v1/models`（OpenAI 端点） | 添加/编辑端点时「刷新」按钮拉全量候选供用户勾选 | 用户点击刷新 |

> 结论：`/v1/models` 路由（问题1）**返回配置态清单**（读库组装），不实时拉上游；现有 `models_cache::fetch_models`（实时拉上游）改服务于「候选拉取」（问题3 的刷新按钮）。

### 3.2 入站格式识别（问题6 的本质）

`handle_proxy` 不是「缺一个路由」——fallback 已能接住任意路径——而是**缺入站格式判断**。改造为：

```
inbound = if path 含 "/chat/completions" { OpenAI } else { Claude }

候选端点：
  Claude 入站 → 全部启用端点（claude 直通 / openai 转换 均支持，现状）
  OpenAI 入站 → 仅过滤出 transformer=openai 的端点（最小版；过滤后为空则 400）

转发与转换矩阵：
  Claude 入站 + claude 端点  → 直通（现状）
  Claude 入站 + openai 端点  → Claude→OpenAI 请求转换 + 响应转回 Claude（现状）
  OpenAI 入站 + openai 端点  → 全透传（新增；不转换请求/响应，upstream=/v1/chat/completions）
  OpenAI 入站 + claude 端点  → 最小版不支持（已被候选过滤排除）
```

---

## 四、数据模型与迁移

### 4.1 `endpoints` 表（新增 2 列）

迁移机制：`migration.rs::MIGRATIONS` 数组末尾**追加一条 v2 脚本**（版本 = 数组下标+1，`run_migrations` 幂等增量执行）。

```sql
-- v2：端点一对多模型 + 端点级代理开关
ALTER TABLE endpoints ADD COLUMN models    TEXT    NOT NULL DEFAULT '[]';
ALTER TABLE endpoints ADD COLUMN use_proxy INTEGER NOT NULL DEFAULT 0;
```

- `models`：JSON 数组字符串（如 `["gpt-5.4","deepseek-r1"]`），Rust 侧 `Vec<String>`，读写经 `serde_json`。
- `use_proxy`：0/1。

### 4.2 `app_config` 表（key-value，无需迁移列）

新增三个键（`config_repo::set_value` / `get_value` 直接读写）：

| 存储 key | AppConfig 字段(camelCase) | 含义 | 进同步白名单? |
|----------|--------------------------|------|--------------|
| `proxyUrl` | `proxyUrl` | 全局代理地址（如 `http://10.0.3.1:7890`） | **否**（设备特定） |
| `openaiUa` | `openaiUa` | 伪装 OpenAI 客户端（Codex 等）的 UA | 是 |
| `claudeCliUa` | `claudeCliUa` | 伪装 claude-cli 的 UA | 是 |

> `SAFE_CONFIG_KEYS`（config_repo.rs）追加 `openaiUa` / `claudeCliUa`；`proxyUrl` 不加入。

---

## 五、后端任务详解（`src-tauri/`）

### P12-1　数据模型与迁移基座
- **层**：Rust（models + storage）
- **文件**：
  - `src-tauri/src/modules/storage/migration.rs`：MIGRATIONS 追加 v2（见 §4.1）。
  - `src-tauri/src/models/endpoint.rs`：`Endpoint` 加 `models: Vec<String>`、`use_proxy: bool`；`CreateEndpointRequest` / `UpdateEndpointRequest` 同步加 `models`（`#[serde(default)]`）、`use_proxy`（Update 为 `Option`）。
  - `src-tauri/src/models/config.rs`：`AppConfig` 加 `proxy_url`、`codex_ua`、`claude_cli_ua`（默认空串）；`Default` 同步。
  - `src-tauri/src/modules/storage/endpoint_repo.rs`：`COLS` 加 `models, use_proxy`；`row_to_endpoint` 反序列化 models（`serde_json::from_str(&s).unwrap_or_default()`）、读 use_proxy；`create` / `update` 的 INSERT/UPDATE 加这两列（models 序列化为字符串存）。
  - `src-tauri/src/modules/storage/config_repo.rs`：`get_config` 用 `parse_str` 读三个新键；`SAFE_CONFIG_KEYS` 追加两个 UA 键。
- **要点**：迁移幂等（已有 v1 库执行 v2 即 ALTER；新库直接到 v2）。models 空数组默认 `'[]'`。
- **前置**：无
- **验收**：旧库升级后 `endpoints` 含新列且默认值正确；`migrations_are_idempotent` 测试通过；`get_config` 返回含三新字段。
- **回链**：#3 #4 #5

### P12-2　端点 CRUD 与 model 语义
- **层**：Rust（commands + storage）
- **文件**：
  - `src-tauri/src/modules/storage/endpoint_repo.rs`：`create` / `update` 持久化 models/use_proxy；`commands/endpoint.rs::clone_endpoint` 复制 models/use_proxy。
  - `src-tauri/src/commands/endpoint.rs`：CRUD 透传新字段（结构已在 P12-1 扩展，命令层多为透传）。
- **要点**：保持 `model`=锁定、`models`=清单 两字段独立存储；不在此任务做转发期的覆盖逻辑（属 P12-5）。
- **前置**：P12-1
- **验收**：创建/更新/克隆端点后 models、use_proxy 正确落库与回读；既有 `crud_and_list_enabled` 测试适配后通过。
- **回链**：#3 #5

### P12-3　候选模型拉取命令（表单刷新用）
- **层**：Rust（commands + modules）
- **文件**：
  - `src-tauri/src/modules/models_cache.rs`：将 `fetch_models(&Endpoint)` 的上游拉取核心抽成可接受 `api_url / api_key / transformer` 的函数（供未保存端点调用）；保留按 `&Endpoint` 的薄封装。
  - `src-tauri/src/commands/models.rs`：新增命令 `fetch_endpoint_models(api_url, api_key, transformer) -> Vec<String>`（仅返回模型 id 列表）。
  - `src-tauri/src/lib.rs`：`invoke_handler!` 注册 `commands::models::fetch_endpoint_models`。
- **要点**：表单里端点可能尚未保存，故按字段传参而非 id。OpenAI 端点拉 `/v1/models`；Claude 端点无标准列举接口，返回空或回落（前端以「自定义输入」兜底）。
- **前置**：无（可与 P12-1 并行）
- **验收**：对一个 OpenAI 兼容地址调用返回真实模型 id 列表；非法地址优雅返回错误而非 panic。
- **回链**：#3

### P12-4　`/v1/models` 路由聚合（配置态公布）
- **层**：Rust（proxy）
- **文件**：
  - `src-tauri/src/modules/proxy/server.rs`：`models_route` 改为 `State<Arc<ProxyState>>`，读库启用端点，按 §3.1「配置态」组装：每个端点 `model` 非空则公布该单模型，否则展开其 `models[]`；空 `models[]` 回落端点默认。输出 `{object:"list", data:[{id, owned_by:端点名, ...}]}`。
  - 复用 `models_cache::model_info` 形态保持字段一致。
- **要点**：读库即可，不请求上游，无需缓存。与 IPC `get_models` 区分（后者仍可保留实时拉取供前端「候选总览」，或在 P12-9 改为读配置态——见该任务）。
- **前置**：P12-1
- **验收**：启用 1 个聚合端点（models 含 N 个）+ 1 个专用端点（model 锁定）后，`curl /v1/models` 返回 N+1 条且 `owned_by` 标注端点名。
- **回链**：#1

### P12-5　OpenAI 入站识别 + 透传（问题6 最小版）
- **层**：Rust（proxy）
- **文件**：
  - `src-tauri/src/modules/proxy/forward.rs::handle_proxy`：
    1. 按 `uri.path()` 判定 `inbound`（含 `/chat/completions` → OpenAI）。
    2. OpenAI 入站时，候选端点过滤为 `transformer=openai`；为空返回 400「无可用 OpenAI 端点」。
    3. 转发分支按 §3.2 矩阵：OpenAI 入站 + openai 端点 → 不转换请求体、`upstream_path` 用入站 path、响应走 `relay_response` 直通。
    4. `model` 锁定覆盖：转发前若 `ep.model` 非空，将请求体 `model` 覆盖为 `ep.model`（两种入站均适用，落实 §二.B）。
  - `src-tauri/src/modules/proxy/resolver.rs`：如需「按入站格式筛候选」可在此加辅助；或在 handle_proxy 内联过滤。
- **要点**：最小版严格不引入 OpenAI→Claude 反向转换。轮换 `max_retries`/故障转移在过滤后的候选集上进行。
- **前置**：P12-1（model 字段语义）
- **验收**：OpenAI SDK 指向网关发 `/v1/chat/completions` 命中 openai 端点，得到正确补全；无 openai 端点时返回明确 400；Claude Code 走 `/v1/messages` 行为不回归。
- **回链**：#6

### P12-6　代理支持（全局地址 + 端点开关 + 双 client）
- **层**：Rust（proxy + config）
- **文件**：
  - `src-tauri/src/modules/proxy/forward.rs`：`ProxyState` 加 `proxy_client: Option<reqwest::Client>`；`send_upstream` 按 `ep.use_proxy && proxy_client.is_some()` 选 client。
  - `src-tauri/src/modules/proxy/server.rs::start_proxy`：读 `proxyUrl` 配置，非空则构建带 `reqwest::Proxy::all(url)` 的 client 存入 `proxy_client`。
  - `src-tauri/src/commands/config.rs::set_config`：`proxyUrl` 变更时按既有 `port_changed` 模式重启代理（重建 client 生效）。
- **要点**：直连 client 与代理 client 各自保留连接池。代理地址非法时记录错误并回落直连，避免整体不可用。
- **前置**：P12-1
- **验收**：配置 `proxyUrl` 后，`useProxy=true` 的端点经代理出网、`false` 的端点直连（可经抓包/日志区分）；改地址即时生效。
- **回链**：#5

### P12-7　UA 注入（按上游格式伪装）
- **层**：Rust（proxy）
- **文件**：
  - `src-tauri/src/modules/proxy/forward.rs::send_upstream`：显式控制 `user-agent`——
    - 目标 openai 端点：`openaiUa` 非空则覆盖，否则透传客户端 UA；
    - 目标 claude 端点：`claudeCliUa` 非空则覆盖，否则透传客户端 UA；
    - 客户端未带 UA 且配置为空：设中性默认（不外泄 `reqwest/x.y.z`）。
  - UA 值来源：`start_proxy` 读取注入 `ProxyState`（与 proxy 同走「配置变更重启代理」）；或 `send_upstream` 每请求读库（二选一，方案推荐随 proxy 重启注入，避免每请求 DB 读）。
  - `src-tauri/src/commands/config.rs::set_config`：`openaiUa`/`claudeCliUa` 变更触发代理重启（若采用注入式）。
- **要点**：覆盖须发生在「复制客户端头」之后，确保最终 UA 唯一且为期望值。
- **前置**：P12-1，建议在 P12-6 之后（共用重启逻辑）
- **验收**：填 `openaiUa` 后命中 openai 端点的上游请求 UA 为该值；清空后恢复透传；无 UA 客户端不再泄露 reqwest 默认 UA。
- **回链**：#4

---

## 六、前端任务详解（`src/`）

### P12-8　端点表单：多选模型 + 刷新拉取 + 代理开关
- **层**：React
- **文件**：
  - `src/services/modules/endpoint.ts`：`Endpoint` 加 `models: string[]`、`useProxy: boolean`；`CreateEndpointRequest` 加 `models?`、`useProxy?`；`endpointApi` 加 `fetchModels(req)`（调 `fetch_endpoint_models`）。
  - `src/pages/Endpoints/_components/EndpointForm.tsx`：`FormState` 加 `models`、`useProxy`；新增「模型」区块（参照 Question.md 截图1）：自定义输入框 + 回车/「+」加入、「刷新」按钮调 `fetchModels` 拉候选、已选标签云（含数量与「清除全部」、单项可删）；新增「启用代理」`Switch`。
- **要点**：刷新拉取的候选合并进可勾选列表；用户手输的自定义模型即使不在候选也可加入。`model`（锁定）与 `models[]` 在 UI 上区分清楚（锁定可做单独输入或从已选中标记）。
- **前置**：P12-2、P12-3
- **验收**：新建聚合端点可刷新拉取并多选 44 个模型保存回读；专用端点填锁定 model 保存生效。
- **回链**：#3

### P12-9　端点展示：分组 + grid 卡片悬停浮层
- **层**：React
- **文件**：
  - `src/pages/Endpoints/_components/EndpointCard.tsx`：grid 形态在「可用性」处挂浮层（`tooltip` 或新增 `hover-card`），内嵌 `scroll-area` **限高可滚动**展示该端点 `models[]`（或锁定 model）。
  - `src/pages/Endpoints/_components/ModelList.tsx`：由「全局平铺」改为「按端点分组」展示配置态模型（或保留为总览，二者择一，建议分组）。
  - 如需新组件：`npx shadcn add hover-card`（现有 ui 已含 `tooltip` / `scroll-area`，可优先复用 tooltip 承载）。
- **要点**：模型数多（数十个）时浮层必须限高滚动，避免撑爆视口（落实 §二.E）。
- **前置**：P12-1（字段）、建议 P12-8 之后
- **验收**：hover 端点可用性弹出该端点模型列表，超长可滚动；list/grid 双视图均正常。
- **回链**：#2

### P12-10　设置页：全局代理地址 + 两个 UA
- **层**：React
- **文件**：
  - `src/services/modules/config.ts`：`AppConfig` 加 `proxyUrl`、`openaiUa`、`claudeCliUa`。
  - `src/pages/Settings/index.tsx`：新增「系统」区块（代理地址 `Input`，对应截图2）与「高级设置」区块（`openaiUa` / `claudeCliUa` 两个 `Input`），复用现有 `Row` + `save({...})` 模式（key 用 camelCase）。
- **要点**：保存即时 `invalidateQueries(["config"])`；UA/代理保存后后端按 P12-6/P12-7 重启代理生效。
- **前置**：P12-1（AppConfig 字段）
- **验收**：填代理地址/UA 后刷新仍在；后端转发按配置生效。
- **回链**：#4 #5

---

## 七、测试与回归

### P12-11　测试与回归校验
- **层**：Rust + React
- **覆盖**：
  - resolver/handle_proxy：OpenAI 入站候选过滤、`model` 锁定覆盖、Claude 入站不回归（Rust `#[cfg(test)]`）。
  - migration：v2 幂等、旧库升级列存在。
  - `/v1/models`：配置态聚合（聚合型展开 + 专用型单条）。
  - 代理：`useProxy` 选 client 正确（可用 mock/日志断言）。
  - UA：覆盖/透传/无 UA 保护三态。
  - 前端：表单多选+刷新、卡片浮层、设置保存（Vitest + mock IPC）。
- **前置**：P12-1 ~ P12-10
- **验收**：上述用例通过；Claude Code 主链路无回归。
- **回链**：#1~#6

---

## 八、构建顺序与依赖

```
P12-1 数据模型与迁移（基座）
  ├─→ P12-2 端点 CRUD/model 语义
  ├─→ P12-4 /v1/models 聚合
  ├─→ P12-5 OpenAI 入站识别
  ├─→ P12-6 代理双 client
  └─→ P12-7 UA 注入（建议接 P12-6）
P12-3 候选拉取命令（可与 P12-1 并行）
P12-2 + P12-3 ─→ P12-8 端点表单
P12-1 ─────────→ P12-9 端点展示（建议接 P12-8）
P12-1 ─────────→ P12-10 设置页
全部 ───────────→ P12-11 测试回归
```

- **里程碑 1（后端闭环）**：P12-1 ~ P12-7 —— 网关支持一对多模型、OpenAI 入站透传、代理、UA。
- **里程碑 2（前端闭环）**：P12-8 ~ P12-10 —— 表单/展示/设置可视化管理。
- **里程碑 3（验收）**：P12-11。

---

## 九、风险与回归点

1. **入站误判**：仅以 path 含 `/chat/completions` 判 OpenAI。若有客户端用非标准路径需复核（当前足够）。
2. **`model` 锁定语义**：锁定会覆盖客户端 model，需在表单明确提示，避免用户误填导致所有请求被改模型。
3. **OpenAI 入站 + 无 openai 端点**：必须返回明确 400，而非静默走 Claude 处理。
4. **配置变更重启代理**：proxy/UA 采用「注入 + 重启」会短暂中断在途请求；与既有 `port` 变更行为一致，可接受。
5. **既有测试夹具**：`endpoint_repo`、`resolver` 测试中的 `Endpoint`/`CreateEndpointRequest` 构造需补 `models`/`use_proxy` 字段，否则编译失败。
6. **shadcn 浮层组件**：若选 `hover-card` 需先 `add`；优先用现有 `tooltip` + `scroll-area` 降低依赖。

---

## 十、验收总清单

- [ ] 旧库平滑升级，新列默认值正确，迁移幂等
- [ ] `/v1/models` 返回各启用端点配置态模型（聚合展开 + 专用单条）
- [ ] OpenAI 客户端 `/v1/chat/completions` 命中 openai 端点正确透传；无 openai 端点明确 400
- [ ] Claude Code `/v1/messages` 主链路无回归
- [ ] 端点表单可刷新拉取候选 + 多选 + 自定义 + 锁定 model + 代理开关
- [ ] grid 卡片悬停浮层显示模型，限高可滚动
- [ ] 设置页代理地址、两个 UA 可配置并生效
- [ ] `useProxy` 端点经代理、其余直连
- [ ] UA 按上游格式覆盖/透传，无 UA 不泄露 reqwest 默认
