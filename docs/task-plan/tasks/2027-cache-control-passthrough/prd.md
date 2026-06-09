# Claude→OpenAI 转换保留 cache_control（显式缓存）

## Goal

让 Claude 客户端经 tauri-gateway 转发到 OpenAI 兼容后端时，保留请求中的 `cache_control` 断点，
使依赖显式缓存（cache_control）的后端能正常建/命中 prompt cache（cached_tokens 不再恒 0）。

## Requirements

- Claude→OpenAI 转换在 **system / 消息文本 / tools** 上保留 `cache_control`，与参考实现 cc-switch 一致。
- 无 cache_control 时行为不变（消息文本仍输出为字符串，向后兼容）。
- 直通路径（Claude→Claude、OpenAI→OpenAI）不受影响（本就 body.clone() 保留）。

## Acceptance Criteria

- [ ] system 为数组且块带一致 cache_control 时，转换后的 system 消息带 cache_control；混合/冲突时不带（保守，对齐 cc-switch）。
- [ ] 消息内容块带 cache_control 时，转换后该消息 content 为 parts 数组且逐块保留 cache_control。
- [ ] 消息内容块无 cache_control 时，content 仍为拼接后的字符串（无回归）。
- [ ] tool 定义带 cache_control 时透传到 OpenAI tools[].cache_control。
- [ ] 既有转换/工具/usage 测试不回归；`cargo test` 通过。

## Definition of Done

实现 + 单测通过；进度写回 progress.csv；scoped 提交；真实"显式缓存命中"无法无头验证，显式声明并给本地核对清单。

## User Stories

- 作为用 Claude Code 经本网关打"支持显式缓存的 OpenAI 兼容后端"的用户，我希望 cache_control 被透传，以便 prompt cache 正常命中、省钱省延迟、用量面板能显示缓存读取。

## Implementation Decisions

- **保留而非剥离 cache_control**：与 cc-switch 一致（其刻意保留、带测试）；对自动缓存后端无害（忽略未知字段），对显式缓存后端有益。与 metadata 任务（刻意不传）相反。
- **cache_control 放置**：system 用**消息级**（sibling of content）；消息文本用 **part 级**（content 数组内）；tools 用 tool 对象级——对齐 cc-switch。
- **system 合并继承规则**：多块 system join 文本；cache_control 仅在全部一致时保留，冲突或混合（有的有有的无）则丢弃。
- **content 形态**：仅当存在 cache_control 才改用数组；否则保持字符串（最小化对现有后端的影响）。
- tool_result 不保留 cache_control（cc-switch 亦不保留）。

## Testing Decisions

- 单测覆盖：system 单块带 cc 保留 / 多块一致保留 / 混合丢弃；消息文本带 cc → 数组且保留、无 cc → 字符串；tool 带 cc 透传。
- 真实后端显式缓存命中无法无头验证，列本地核对清单。

## Out of Scope

- tool_result 的 cache_control（不保留）。
- 流式响应侧改动（cache 读取已由 usage.rs 处理）。
- 标准 OpenAI 自动缓存逻辑（不依赖 cache_control）。

## Technical Notes

- 参考 cc-switch `providers/transform.rs`：system :144-158、合并 :283-344、消息文本 :380-387/:458-473、tools :218-220。
- 本项目落点与设计见 research/cache-control-handling.md。
