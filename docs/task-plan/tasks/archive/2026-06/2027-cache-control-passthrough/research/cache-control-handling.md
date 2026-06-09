# cache_control 透传调研与设计

> 结论：**是真实缺口**。tauri-gateway 在 Claude→OpenAI 转换时丢弃 `cache_control`，导致依赖显式缓存（cache_control）的兼容后端无法建缓存（cached_tokens 恒 0）。参考实现 cc-switch **刻意保留** cache_control。用户选「第三方/不确定」后端 → 保留 cache_control 最稳妥（自动缓存后端无害、显式缓存后端有益）。

## 1. 我方现状（丢弃 cache_control）

`src-tauri/src/modules/transform/`：
- **system**：`types.rs:extract_system_text` 只抽 `.text` 用 `\n` 拼成纯串 → cache_control 丢失（claude_openai.rs:57-63）。
- **消息文本**：`claude_openai.rs:convert_claude_message_to_openai` 把 text 块 `push_str` 拼成串 → cache_control 丢失（:111-116, :145-159）。
- **tools**：`claude_openai.rs:71-94` 只构 `{type,function:{name,description,parameters}}` → cache_control 丢失。
- 直通路径（Claude→Claude、OpenAI→OpenAI）走 `body.clone()`，cache_control 不丢；问题仅在 **Claude→OpenAI 转换路径**。

## 2. cc-switch 做法（保留 cache_control，权威参考）

`E:\myCode\cc-switch\src-tauri\src\proxy\providers\transform.rs::anthropic_to_openai_with_reasoning_content`：
- **system**（:144-158）：数组逐块发 `{role:system, content:text, [cache_control]}`（**消息级** cache_control，注释 "preserve cache_control for compatible proxies"）。
- **system 合并**（`normalize_openai_system_messages` :283-344）：多条 system 合并为一条；cache_control **继承规则**——全部一致才保留；**冲突或"有的有有的没有"则丢弃**（带测试，保守）。
- **消息文本**（`convert_message_to_openai` :380-387）：text 块 → content part `{type:text,text,[cache_control]}`（**part 级** cache_control）。
- **content 形态**（:458-473）：含 cache_control 或多 part → content 用**数组**；否则退化为字符串。
- **tools**（:218-220）：保留 tool 上的 cache_control。
- 测试：`test_anthropic_to_openai_cache_control_preserved`（:1202）、`..._preserves_matching_system_cache_control_when_merging`、`..._drops_mixed_present_absent_..._when_merging`、`..._drops_conflicting_...`。

## 3. 本项目落地设计（移植 cc-switch，适配现有结构）

- **system**：新增 `claude_system_to_openai(&Value)->Option<Value>` 取代 `extract_system_text`：
  - string → `{role:system, content:s}`（空则 None）。
  - array → join 各块 text；按 cc-switch 继承规则收集 cache_control（一致→挂消息级 cache_control；冲突/混合→不挂）。
  - 移除 `types.rs::extract_system_text`（仅此一处用）。
- **消息文本**：改 `convert_claude_message_to_openai` —— text 块收成 `content_parts`（`{type:text,text,[cache_control]}`）；组装时：任一 part 有 cache_control → content=数组；否则 join 成字符串（向后兼容）。只收非空 text（与原 has_text 语义一致）。
- **tools**：tools 循环里若 `t.get("cache_control")` 存在则挂到 tool 对象。
- tool_result 不保留 cache_control（cc-switch 也不保留）。

## 4. 根因分型（已与用户确认：第三方/不确定）

- 标准 OpenAI 自动缓存：丢 cache_control 无影响（缓存自动、读取已由 usage.rs 修）。
- 显式缓存后端（套 OpenAI 格式的 Claude 中转等）：丢 cache_control = 不建缓存 = cached_tokens 0 = 不显示 → 本修复直接解决。
- 保留 cache_control 对两类后端都不会更差（自动缓存后端忽略未知字段）。

## 5. 无法无头验证

真实"显式缓存命中"需用会认 cache_control 的真实后端实测：抓转发到上游的请求体确认 cache_control 在位、看响应 `prompt_tokens_details.cached_tokens`/`cache_read_input_tokens`>0。本环境只能验证转换函数产出（单测）。
