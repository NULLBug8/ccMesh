use serde_json::Value;

/// 流式转换上下文：OpenAI chunk 流 → Claude SSE 事件的累积状态。
#[derive(Default)]
pub struct StreamContext {
    pub message_start_sent: bool,
    pub text_block_started: bool,
    pub thinking_block_started: bool,
    pub tool_block_started: bool,
    /// 当前内容块索引（每开启一个块自增）。
    pub block_index: i64,
    pub current_tool_id: String,
    pub current_tool_name: String,
    pub tool_arguments: String,
    pub message_stop_sent: bool,
    pub model_name: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

/// 提取 system 文本：字符串原样；数组取各块 `.text` 以换行连接。
pub fn extract_system_text(system: &Value) -> String {
    match system {
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr
            .iter()
            .filter_map(|b| b.get("text").and_then(|t| t.as_str()))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
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
        other => other.to_string(),
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

/// 构造一条 Claude SSE 事件文本（`event: <type>\ndata: <json>\n\n`）。
pub fn build_claude_event(event: &str, data: &Value) -> String {
    format!("event: {event}\ndata: {data}\n\n")
}
