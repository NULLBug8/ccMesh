## 流式多工具：按 OpenAI index 路由

> 一句话结论：并行 tool call 必须用 index→状态 映射，勿用单一 current_tool 指针

**你会遇到这个问题的场景**
OpenAI 流式 `tool_calls`  delta 中，同一轮 assistant 消息可能并行发出多个 function call，每个 delta 带 `index` 字段。网关需转换为 Anthropic 的多个 `tool_use` content block。

**为什么会出错**
单全局变量 `current_tool_id` / `current_tool_name` 在第二个 tool 的第一个 delta 到达时会覆盖第一个 tool 的状态，导致参数串味、block 未关闭、或 `content_block_start` 重复/遗漏。

**正确做法**
- 维护 `HashMap<openai_index, ToolBlockState>`（实现中常命名 `ToolBlock`），键为 OpenAI 的 `index`
- 首次见到某 index 时才发 `content_block_start`，记录 anthropic 侧 content index
- `function.arguments` delta 按 index 路由到对应块的 `input_json_delta`
- finish 时遍历 map，关闭所有仍 open 的 tool block
- 与 `next_content_index` 配合，保证 Anthropic 侧 block 序号单调

**反例**
❌ 错误：每个 delta 更新同一个 `state.arguments`  
✅ 正确：`tool_blocks_by_index.get_mut(&index).arguments.push_str(delta)`

---
_最后更新：2026-06-28_
