# 2027 Claude→OpenAI 转换保留 cache_control

## 目标

Claude→OpenAI 转换在 system / 消息文本 / tools 上保留 `cache_control`，使显式缓存后端能建/命中缓存。

## 现状（根因）

`claude_request_to_openai` 及其辅助把 cache_control 丢了：system 经 `extract_system_text` 拍平为纯串；消息文本 `push_str` 拼接；tools 不带 cache_control。仅影响 Claude→OpenAI 转换路径（直通不受影响）。详见 research/cache-control-handling.md。

## 关键文件/落点

- `src-tauri/src/modules/transform/claude_openai.rs`
  - 新增 `claude_system_to_openai(&Value) -> Option<Value>`（system 保留+合并继承 cache_control），替换 :57-63 对 `extract_system_text` 的调用。
  - 改 `convert_claude_message_to_openai`（:99-163）：text 块收成 `content_parts`（含 cache_control），content 按"有 cc→数组 / 无 cc→字符串"输出。
  - 改 tools 循环（:71-94）：透传 tool 的 cache_control。
- `src-tauri/src/modules/transform/types.rs`：移除仅此处使用的 `extract_system_text`（:34-44）+ claude_openai.rs 的对应 import。

## 任务拆解

- **2027.1 [集成] system 保留 cache_control** —— 新增 `claude_system_to_openai`，替换 extract_system_text 调用；移除 extract_system_text。
- **2027.2 [集成] 消息文本保留 cache_control** —— 改 convert_claude_message_to_openai：content_parts + 数组/字符串输出。
- **2027.3 [集成] tools 保留 cache_control** —— tools 循环透传 cache_control。
- **2027.4 [测试] 单测 + 回归** —— 新增 cache_control 用例；跑 cargo test。

## 数据契约

```
// system 数组（输入）→ 输出
[{type:text,text:"A"},{type:text,text:"B",cache_control:{type:ephemeral}}]  // 混合
  → {role:system, content:"A\nB"}                      // 混合：不挂 cache_control
[{type:text,text:"S",cache_control:{type:ephemeral}}]
  → {role:system, content:"S", cache_control:{type:ephemeral}}

// 消息内容块（输入）→ 输出 content
[{type:text,text:"hi"}]                                // 无 cc
  → "hi"                                               // 字符串（向后兼容）
[{type:text,text:"hi",cache_control:{type:ephemeral}}] // 有 cc
  → [{type:text,text:"hi",cache_control:{type:ephemeral}}]  // 数组，逐块保留

// tools[]（输入带 cache_control）→ OpenAI tools[].cache_control 透传
```

## 验收标准

见 prd.md Acceptance Criteria。

## 测试点

- claude_system_to_openai：string / array 一致 cc 保留 / array 混合丢弃 / array 冲突丢弃 / 空。
- convert_claude_message_to_openai：text 带 cc→数组保留；多 text 无 cc→字符串拼接；text+tool_use 混合；tool_result 不带 cc。
- tools：带 cache_control 透传；不带则无该字段。
- 回归：现有 request_* / response_* / streaming / usage 测试全绿。

## 提交策略（scoped）

1. `docs(task-plan)`: prd/feature/research/progress（2027 任务）。
2. `feat(transform)`: claude_openai.rs + types.rs（system/消息/tools 保留 cache_control）+ 测试。

派生 scoped-commit-bot，传精确文件清单；不碰其它文件；不推送。

## Run（验证）

- `cargo check` / `cargo test --lib claude_openai` / `cargo test --lib`（src-tauri/）。
- **无法无头验证**：真实显式缓存命中需用会认 cache_control 的真实后端实测（抓上游请求体看 cache_control 在位、看响应 cached_tokens>0）。
