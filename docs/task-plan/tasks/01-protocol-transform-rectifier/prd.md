# 协议转换工具兼容性增强（Rectifier 修复器）

## Goal

把 cc-switch 验证过的协议转换健壮性做法，**据其真实源码**集成进本项目的 Claude↔OpenAI 转换与转发层，
消除第三方 OpenAI/Claude 兼容后端在工具调用、thinking 签名、流式多工具上的兼容性 400/损坏问题。

## Requirements

- **A 主动 canonicalization**：请求转换构建上游工具调用时，对 `arguments` 做 JSON 规范化（排序键，稳定序列化以提升 prefix-cache 命中），且空 `arguments` 整串强制为 `"{}"`，避免严格后端（如 Minimax）报 `invalid function arguments json string` 400。
- **B thinking 签名 reactive rectifier**：上游返回 thinking 签名类错误时，读取错误体、匹配错误消息、原地移除请求体中 thinking/redacted_thinking 块与遗留 signature 字段后**单次静默重试**，对客户端透明。
- **C 流式多工具 by index**：流式响应按 OpenAI `tool_calls[].index` 区分多个工具调用，正确映射为 Anthropic 顺序 content block，修复当前"单工具、按 id 判定"导致并行/多工具流式损坏的缺陷。

## Acceptance Criteria

- [ ] 工具调用 `input={}` 或缺失时，上游请求的 `arguments` 为 `"{}"` 而非 `""`。
- [ ] 相同语义、键顺序不同的工具参数，规范化后产出**逐字节一致**的 `arguments` 串。
- [ ] 纯文本（非 JSON）参数串不被破坏，原样保留。
- [ ] 上游返回 `Invalid \`signature\` in \`thinking\` block` 等 7 类签名错误（含嵌套 JSON 错误体）时，自动移除 thinking/signature 后重试；重试至多一次，不死循环。
- [ ] 签名 rectify 不命中或重试仍失败时，错误体原样回传客户端（不吞错、不改状态码）。
- [ ] 一个流式响应里出现 2 个及以上工具调用（不同 index）时，每个工具各自成块、参数不串味；stop_reason 正确为 `tool_use`。
- [ ] 既有文本流/单工具流/usage 行为不回归。
- [ ] `cargo test` / `cargo check` 通过。

## Definition of Done

A/B/C 三项实现并通过单测；现有转换与转发测试全绿；进度写回 progress.csv；按模块 scoped 提交；
无法无头验证的真实出网行为（真打第三方后端）显式声明并给本地核对清单。

## User Stories

- 作为 Claude Code 用户，我希望工具调用经代理转发到第三方 OpenAI 兼容后端时不因空 `arguments` 被拒，以便工具链稳定可用。
- 作为使用第三方 Claude 兼容中转的用户，我希望 thinking 签名不匹配时代理自动修复重试，以便无感继续对话而非报错中断。
- 作为依赖并行工具调用的用户，我希望流式响应里多个工具调用不互相串味，以便多工具结果正确解析。
- 作为运维，我希望参数规范化提升 prefix-cache 命中，以便降低上游成本与延迟。

## Implementation Decisions

- **据真实 cc-switch 源码集成**，不依据需求摘要的二手描述（摘要与源码有出入，已在 research/cc-switch-real-design.md 记录）。
- **空参数走主动 canonicalization，不走反应式字段剥离**：与 cc-switch 一致，空处理只在 `arguments` 整串层面（`""`→`"{}"`），**不删除** `{"pages":""}` 这类字段级空值（用户已确认放弃字段剥离）。
- **canonicalization 语义**：对象键递归排序；空/纯空白 arguments 串→`"{}"`；可解析为 JSON 的串规范化、不可解析的纯文本原样保留。
- **rectifier 作用域**：thinking 签名 rectify 操作 **Anthropic 请求体**，对 Claude→Claude passthrough 路径最有效；OpenAI-transform 路径 thinking 已被转换处理，命中概率低但逻辑统一保留。
- **错误匹配方式**：对错误体**原始文本小写 substring 匹配**（cc-switch 测试证明嵌套 JSON 错误体亦可命中，无需结构化解析）。
- **重试契约**：签名 rectify 一次性（单标志位），`applied==false` 不重试，重试仍失败按普通错误回传。
- **配置**：引入精简 `RectifierConfig{enabled, request_thinking_signature}`（砍掉 cc-switch 的 budget/media 子开关），默认开启。
- **放弃会话级 ID 缓存（D）**：cc-switch 的 ID 缓存是 Codex Responses API→Chat 桥接专属，本项目 Claude↔OpenAI Chat 路径客户端每轮发完整历史、ID 自洽，建之属过度设计。
- **流式 index 为借鉴非照搬**：沿用 cc-switch "OpenAI index→Anthropic block 状态映射"思路，落到本项目 `StreamConverter`/`StreamContext`，不引入其异步 stream 架构。
- **保持 Value-based**：不为本任务把转换层重构为类型化 struct（KISS，最小改动面）。

## Testing Decisions

- 单测覆盖：canonicalization（排序一致/空→{}/纯文本保留）、签名检测 7 类（含嵌套 JSON 错误体）、rectify 原地修改结果、流式双工具不串味、文本/单工具回归。
- canonicalization 与 rectifier 纯逻辑函数优先单测；流式经 `StreamConverter` 驱动断言 SSE 文本。
- 真实出网（实打第三方后端触发 400/签名错误）**无法无头验证**，列本地核对清单。

## Out of Scope

- 字段级空值剥离（`{"pages":""}`）—— 用户确认不做。
- 会话级 call_id↔tool_use_id 缓存（D）—— Codex 专属，不适用。
- thinking budget rectifier、media fallback rectifier —— 本轮不做（可后续按需补）。
- Codex Responses API / Gemini 等其它上游格式 —— 不在本项目范围。
- 转换层类型化 struct 重构。

## Technical Notes

- 参考源码全部位于 `E:\myCode\cc-switch\src-tauri\src\proxy\`，关键文件与行号见 research/cc-switch-real-design.md。
- 本项目落点与现状见 research/current-transform-proxy.md。
- 流式规范化只能在工具参数**整块收尾**做（增量 `partial_json` 片段不可逐片排序），实现时注意。
- forward.rs 错误体当前从不读取（:401-408 直接 relay_passthrough）；B 需要先缓冲读取错误体，consume 掉 resp 后回传须用缓冲字节重建 Response。
