# 01 协议转换工具兼容性增强（Rectifier 修复器）

## 目标

据真实 cc-switch 源码，把三项健壮性能力落进本项目转换/转发层：A 主动 canonicalization、B thinking 签名 reactive rectifier、C 流式多工具 by index。

## 现状（根因）

- 工具参数 `arguments` 用 `serde_json::to_string(&input)`，**不排序、空 input 产出非稳定串**（claude_openai.rs:124）。
- forward.rs 对 4xx **从不读错误体**，400/401 直接 `relay_passthrough` 原样回传（forward.rs:401-408）→ 无任何反应式修复。
- 流式工具按 `id` 出现判定、单 `current_tool_*`、忽略 `index`（streaming.rs:139-179, types.rs:12-14）→ 多工具串味风险。

详见 `research/current-transform-proxy.md` 与 `research/cc-switch-real-design.md`。

## 关键文件/落点

**新增**
- `src-tauri/src/modules/transform/json_canonical.rs` —— 移植 cc-switch json_canonical.rs（去 sha2/hash 部分，只留 canonicalize_value / canonical_json_string / canonicalize_json_string_if_parseable / canonicalize_tool_arguments_str / canonicalize_tool_arguments）。
- `src-tauri/src/modules/transform/thinking_rectifier.rs` —— 移植 should_rectify_thinking_signature + rectify_anthropic_request + should_remove_top_level_thinking + RectifyResult + 精简 RectifierConfig{enabled, request_thinking_signature}。

**修改**
- `transform/mod.rs` —— 加 `pub mod json_canonical;` `pub mod thinking_rectifier;`。
- `transform/claude_openai.rs` —— :124 arguments 改 canonicalize；:131-140 tool_result 结构化 content 改 canonical_json_string。
- `transform/types.rs` —— StreamContext 增 `tool_blocks: HashMap<i64, ToolBlock>`（及 ToolBlock 结构）；保留或弃用单 current_tool_* 视实现。
- `transform/streaming.rs` —— handle_tool_call 改为按 index 路由；close/finish 关闭所有开着的 tool 块；工具参数收尾可选 canonicalize。
- `proxy/forward.rs` —— 重试循环加签名 rectifier：缓冲读 4xx 错误体→匹配→rectify body_json→单次重试；不命中/已重试则用缓冲字节回传。
- `proxy/server.rs` 或 `ProxyState`（forward.rs:68-85）—— 注入 RectifierConfig（默认 enabled）。

## 任务拆解

- **01.1 [纯逻辑] json_canonical 移植** —— 新建 json_canonical.rs + 单测（排序一致/空→{}/纯文本保留/字段变体）。
- **01.2 [纯逻辑] thinking_rectifier 移植** —— 新建 thinking_rectifier.rs + RectifierConfig + 单测（7 类检测含嵌套 JSON、rectify 原地修改计数、顶层 thinking 删除条件）。
- **01.3 [集成] 请求转换接入 canonicalization** —— 改 claude_openai.rs:124/131-140；更新/新增请求侧测试（arguments 稳定、空→{}）。
- **01.4 [集成] 流式多工具 by index** —— 改 types.rs StreamContext + streaming.rs handle_tool_call/close/finish；新增双工具流式不串味测试；回归文本/单工具/usage。
- **01.5 [转发] 签名 rectifier 接入 forward.rs** —— 错误体缓冲读取 + 匹配 + rectify body_json + 单次重试 + 缓冲回传；ProxyState/server 注入 RectifierConfig。
- **01.6 [验证] 整体回归** —— cargo check/test/build；本地核对清单（真实出网部分）。

## 数据契约

```rust
// thinking_rectifier.rs
pub struct RectifierConfig { pub enabled: bool, pub request_thinking_signature: bool } // Default 全 true
pub struct RectifyResult { pub applied: bool, pub removed_thinking_blocks: usize,
    pub removed_redacted_thinking_blocks: usize, pub removed_signature_fields: usize }
pub fn should_rectify_thinking_signature(error_message: Option<&str>, cfg: &RectifierConfig) -> bool;
pub fn rectify_anthropic_request(body: &mut serde_json::Value) -> RectifyResult;

// json_canonical.rs（模块内 pub）
pub fn canonical_json_string(v: &Value) -> String;
pub fn canonicalize_tool_arguments_str(s: &str) -> String;       // 空/空白 -> "{}"
pub fn canonicalize_tool_arguments(v: Option<&Value>) -> String;  // String/结构化/缺失

// types.rs StreamContext 新增
struct ToolBlock { anthropic_index: i64, id: String, name: String, started: bool }
tool_blocks: HashMap<i64 /*openai tool index*/, ToolBlock>
```

forward.rs 重试循环关键约束：
- `body_json` 改为可变（或引入 `rectify_body: Option<Value>` 覆盖变量），rectify 后下一轮顶部重建 attempt_body 自然生效。
- 一次性标志 `sig_rectified: bool`，置位后不再 rectify。
- 4xx 分支：`let bytes = resp.bytes().await?` 缓冲；`should_rectify(&String::from_utf8_lossy(&bytes), cfg)` 命中且未重试 → rectify + continue；否则用 bytes 重建 Response（带原 status/headers）回传。

## 验收标准

见 prd.md Acceptance Criteria（逐条对应 01.1~01.5 的测试）。

## 测试点

- json_canonical：键排序逐字节一致；`""`/`"   "`→`"{}"`；纯文本保留；None/空串/结构化变体。
- thinking_rectifier：7 类错误命中（含嵌套 JSON 串）；config 关闭不命中；rectify 删块/删 signature 计数正确；顶层 thinking 删除条件（enabled+末assistant首块非thinking+含tool_use）。
- claude_openai：tool_use input={} → arguments "{}"；键序无关稳定；tool_result 结构化 content 规范化。
- streaming：双工具不同 index 各自成块、参数不串味、stop_reason=tool_use；单工具/文本/usage 回归。
- forward（尽量单测/集成）：签名错误体触发 rectify+重试；不命中原样回传；重试一次上限。

## 提交策略（scoped，按模块分组，先 docs 再逻辑再集成再转发）

1. `docs(task-plan)`: prd.md + feature.md + research/* + progress.csv 新行。
2. `feat(transform)`: json_canonical.rs + 测试（01.1）。
3. `feat(transform)`: thinking_rectifier.rs + RectifierConfig + 测试（01.2）。
4. `feat(transform)`: claude_openai.rs canonicalization 接入 + 测试（01.3）。
5. `feat(transform)`: types.rs + streaming.rs 多工具 index + 测试（01.4）。
6. `feat(proxy)`: forward.rs 签名 rectifier + ProxyState/server 注入（01.5）。

每组只 `git add` 精确文件，提交前 `git status --short` 核对。派生 scoped-commit-bot 执行。

## Run（验证命令，按 Cargo 探测）

- 类型检查：`cargo check`（在 src-tauri/）
- 单测：`cargo test`（库测试；如需过滤 `cargo test transform::`）
- 构建：`cargo build`
- **无法无头验证**：真实打第三方 OpenAI/Claude 兼容后端触发空 arguments 400 / thinking 签名 400 / 并行工具流式——需本地用真实端点 + `run`/`verify` 或手动核对。
