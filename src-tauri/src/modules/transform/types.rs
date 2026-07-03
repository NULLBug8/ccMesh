use serde_json::Value;
use std::collections::HashMap;

use crate::modules::transform::json_canonical::canonical_json_string;

/// 流式工具块：OpenAI `tool_calls[].index` → Anthropic content block 的映射状态。
#[derive(Default, Clone)]
pub struct ToolBlock {
    /// 该工具调用对应的 Anthropic content block 索引。
    pub anthropic_index: i64,
}

/// 流式转换上下文：OpenAI chunk 流 → Claude SSE 事件的累积状态。
#[derive(Default)]
pub struct StreamContext {
    pub message_start_sent: bool,
    pub text_block_started: bool,
    pub thinking_block_started: bool,
    /// 是否已开启过任意工具块（用于 stop_reason 判定）。
    pub any_tool_started: bool,
    /// 下一个可用的 Anthropic content block 索引（开启新块时分配并自增）。
    pub block_index: i64,
    /// OpenAI tool index → 工具块状态，支持单个响应内多个（并行）工具调用。
    pub tool_blocks: HashMap<i64, ToolBlock>,
    pub message_stop_sent: bool,
    pub model_name: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
}

/// 提取 tool_result content：字符串原样；数组取 `.text` 以换行连接。
pub fn extract_tool_result_content(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr
            .iter()
            .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Null => String::new(),
        other => canonical_json_string(other),
    }
}

/// OpenAI `finish_reason` → Claude `stop_reason`。
pub fn map_finish_reason(finish: Option<&str>, has_tool: bool) -> &'static str {
    if has_tool {
        return "tool_use";
    }
    match finish {
        Some("length") => "max_tokens",
        Some("tool_calls") => "tool_use",
        Some("stop") => "end_turn",
        _ => "end_turn",
    }
}

/// 构造一条 SSE 事件文本（`event: <type>\ndata: <json>\n\n`）。Claude 与 Responses 流共用。
pub fn build_sse_event(event: &str, data: &Value) -> String {
    format!("event: {event}\ndata: {data}\n\n")
}

/// 构造一条 Claude SSE 事件文本。
pub fn build_claude_event(event: &str, data: &Value) -> String {
    build_sse_event(event, data)
}
