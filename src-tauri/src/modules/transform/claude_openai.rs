use serde_json::{json, Value};

use crate::error::AppResult;
use crate::modules::transform::transformer::Transformer;
use crate::modules::transform::types::{
    extract_system_text, extract_tool_result_content, map_finish_reason,
};

/// Claude↔OpenAI Chat 转换器（请求 / 非流式响应；流式见 [`super::streaming`] —— P2-5 接入）。
pub struct ClaudeOpenAiTransformer;

impl Transformer for ClaudeOpenAiTransformer {
    fn transform_request(&self, req: &Value, endpoint_model: Option<&str>) -> AppResult<Value> {
        Ok(claude_request_to_openai(req, endpoint_model))
    }
}

/// Claude Messages 请求 → OpenAI Chat 请求。
pub fn claude_request_to_openai(claude: &Value, endpoint_model: Option<&str>) -> Value {
    let mut out = serde_json::Map::new();

    // model：端点配置优先，否则用请求里的 model
    let model = endpoint_model
        .filter(|m| !m.trim().is_empty())
        .map(|m| m.to_string())
        .or_else(|| {
            claude
                .get("model")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string())
        });
    if let Some(m) = model {
        out.insert("model".into(), json!(m));
    }

    if let Some(mt) = claude.get("max_tokens").and_then(|v| v.as_i64()) {
        if mt > 0 {
            out.insert("max_completion_tokens".into(), json!(mt));
        }
    }
    if let Some(t) = claude.get("temperature").and_then(|v| v.as_f64()) {
        if t > 0.0 {
            out.insert("temperature".into(), json!(t));
        }
    }
    let stream = claude
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if stream {
        out.insert("stream".into(), json!(true));
        out.insert("stream_options".into(), json!({ "include_usage": true }));
    }

    // messages（system 前置）
    let mut messages: Vec<Value> = Vec::new();
    if let Some(sys) = claude.get("system") {
        let s = extract_system_text(sys);
        if !s.is_empty() {
            messages.push(json!({ "role": "system", "content": s }));
        }
    }
    if let Some(arr) = claude.get("messages").and_then(|v| v.as_array()) {
        for msg in arr {
            convert_claude_message_to_openai(msg, &mut messages);
        }
    }
    out.insert("messages".into(), json!(messages));

    // tools + tool_choice
    if let Some(tools) = claude.get("tools").and_then(|v| v.as_array()) {
        let mut otools = Vec::new();
        for t in tools {
            let name = t.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if name.is_empty() {
                continue;
            }
            otools.push(json!({
                "type": "function",
                "function": {
                    "name": name,
                    "description": t.get("description").cloned().unwrap_or(json!("")),
                    "parameters": t.get("input_schema").cloned().unwrap_or(json!({ "type": "object" })),
                }
            }));
        }
        if !otools.is_empty() {
            out.insert("tools".into(), json!(otools));
            out.insert(
                "tool_choice".into(),
                convert_tool_choice(claude.get("tool_choice")),
            );
        }
    }

    Value::Object(out)
}

fn convert_claude_message_to_openai(msg: &Value, out: &mut Vec<Value>) {
    let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("user");
    match msg.get("content") {
        Some(Value::String(s)) => {
            out.push(json!({ "role": role, "content": s }));
        }
        Some(Value::Array(blocks)) => {
            let mut text = String::new();
            let mut tool_calls: Vec<Value> = Vec::new();
            let mut tool_results: Vec<Value> = Vec::new();

            for b in blocks {
                match b.get("type").and_then(|v| v.as_str()).unwrap_or("") {
                    "text" => {
                        if let Some(t) = b.get("text").and_then(|v| v.as_str()) {
                            text.push_str(t);
                        }
                    }
                    "tool_use" => {
                        let id = b.get("id").and_then(|v| v.as_str()).unwrap_or("");
                        let name = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        if id.is_empty() || name.is_empty() {
                            continue;
                        }
                        let input = b.get("input").cloned().unwrap_or(json!({}));
                        let args = serde_json::to_string(&input).unwrap_or_else(|_| "{}".into());
                        tool_calls.push(json!({
                            "id": id,
                            "type": "function",
                            "function": { "name": name, "arguments": args }
                        }));
                    }
                    "tool_result" => {
                        let tid = b.get("tool_use_id").and_then(|v| v.as_str()).unwrap_or("");
                        let c = extract_tool_result_content(
                            b.get("content").unwrap_or(&Value::Null),
                        );
                        tool_results.push(json!({
                            "role": "tool",
                            "tool_call_id": tid,
                            "content": c
                        }));
                    }
                    _ => {}
                }
            }

            let has_text = !text.is_empty();
            let has_calls = !tool_calls.is_empty();
            if has_text || has_calls {
                let mut m = serde_json::Map::new();
                m.insert("role".into(), json!(role));
                if has_text || !has_calls {
                    m.insert("content".into(), json!(text));
                }
                if has_calls {
                    m.insert("tool_calls".into(), json!(tool_calls));
                }
                out.push(Value::Object(m));
            }
            out.extend(tool_results);
        }
        _ => {}
    }
}

fn convert_tool_choice(tc: Option<&Value>) -> Value {
    match tc {
        Some(Value::String(s)) => json!(s),
        Some(Value::Object(o)) => match o.get("type").and_then(|v| v.as_str()) {
            Some("tool") => {
                let name = o.get("name").and_then(|v| v.as_str()).unwrap_or("");
                json!({ "type": "function", "function": { "name": name } })
            }
            Some("any") => json!("required"),
            _ => json!("auto"),
        },
        _ => json!("auto"),
    }
}

/// OpenAI Chat（非流式）响应 → Claude Messages 响应。
pub fn openai_response_to_claude(resp: &Value) -> Value {
    let mut content: Vec<Value> = Vec::new();
    let mut stop_reason = "end_turn".to_string();

    let choice = resp
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first());

    if let Some(ch) = choice {
        let msg = ch.get("message");

        if let Some(text) = msg
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
        {
            if !text.is_empty() {
                content.extend(split_think_tagged_text(text));
            }
        }

        let mut has_tool = false;
        if let Some(tcs) = msg
            .and_then(|m| m.get("tool_calls"))
            .and_then(|t| t.as_array())
        {
            for tc in tcs {
                let id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let func = tc.get("function");
                let name = func
                    .and_then(|f| f.get("name"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let args_str = func
                    .and_then(|f| f.get("arguments"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("{}");
                let input: Value = serde_json::from_str(args_str).unwrap_or(json!({}));
                content.push(json!({
                    "type": "tool_use", "id": id, "name": name, "input": input
                }));
                has_tool = true;
            }
        }

        let fr = ch.get("finish_reason").and_then(|v| v.as_str());
        stop_reason = map_finish_reason(fr, has_tool).to_string();
    }

    let usage = resp.get("usage");
    let input_tokens = usage
        .and_then(|u| u.get("prompt_tokens"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let output_tokens = usage
        .and_then(|u| u.get("completion_tokens"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    json!({
        "id": resp.get("id").cloned().unwrap_or(json!("")),
        "type": "message",
        "role": "assistant",
        "model": resp.get("model").cloned().unwrap_or(json!("")),
        "content": content,
        "stop_reason": stop_reason,
        "stop_sequence": Value::Null,
        "usage": { "input_tokens": input_tokens, "output_tokens": output_tokens }
    })
}

/// 将含 `<think>...</think>` 标签的文本拆分为 text / thinking 内容块。
pub fn split_think_tagged_text(text: &str) -> Vec<Value> {
    let mut blocks = Vec::new();
    let mut rest = text;

    while let Some(start) = rest.find("<think>") {
        let before = &rest[..start];
        if !before.is_empty() {
            blocks.push(json!({ "type": "text", "text": before }));
        }
        let after = &rest[start + "<think>".len()..];
        match after.find("</think>") {
            Some(end) => {
                let thinking = &after[..end];
                blocks.push(json!({ "type": "thinking", "thinking": thinking }));
                rest = &after[end + "</think>".len()..];
            }
            None => {
                blocks.push(json!({ "type": "thinking", "thinking": after }));
                return blocks;
            }
        }
    }
    if !rest.is_empty() {
        blocks.push(json!({ "type": "text", "text": rest }));
    }
    if blocks.is_empty() {
        blocks.push(json!({ "type": "text", "text": "" }));
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_maps_system_and_max_tokens() {
        let claude = json!({
            "model": "claude-3",
            "max_tokens": 100,
            "system": "be brief",
            "messages": [{ "role": "user", "content": "hi" }]
        });
        let out = claude_request_to_openai(&claude, Some("gpt-4o"));
        assert_eq!(out["model"], json!("gpt-4o"));
        assert_eq!(out["max_completion_tokens"], json!(100));
        assert_eq!(out["messages"][0]["role"], json!("system"));
        assert_eq!(out["messages"][0]["content"], json!("be brief"));
        assert_eq!(out["messages"][1]["content"], json!("hi"));
    }

    #[test]
    fn request_stream_sets_include_usage() {
        let claude = json!({ "stream": true, "messages": [] });
        let out = claude_request_to_openai(&claude, None);
        assert_eq!(out["stream"], json!(true));
        assert_eq!(out["stream_options"]["include_usage"], json!(true));
    }

    #[test]
    fn request_tool_use_becomes_tool_calls() {
        let claude = json!({
            "messages": [{
                "role": "assistant",
                "content": [
                    { "type": "text", "text": "calling" },
                    { "type": "tool_use", "id": "t1", "name": "get_weather", "input": { "city": "SF" } }
                ]
            }]
        });
        let out = claude_request_to_openai(&claude, None);
        let m = &out["messages"][0];
        assert_eq!(m["content"], json!("calling"));
        assert_eq!(m["tool_calls"][0]["id"], json!("t1"));
        assert_eq!(m["tool_calls"][0]["function"]["name"], json!("get_weather"));
        assert_eq!(m["tool_calls"][0]["function"]["arguments"], json!("{\"city\":\"SF\"}"));
    }

    #[test]
    fn request_tool_result_becomes_tool_message() {
        let claude = json!({
            "messages": [{
                "role": "user",
                "content": [{ "type": "tool_result", "tool_use_id": "t1", "content": "sunny" }]
            }]
        });
        let out = claude_request_to_openai(&claude, None);
        assert_eq!(out["messages"][0]["role"], json!("tool"));
        assert_eq!(out["messages"][0]["tool_call_id"], json!("t1"));
        assert_eq!(out["messages"][0]["content"], json!("sunny"));
    }

    #[test]
    fn response_maps_text_and_usage() {
        let resp = json!({
            "id": "x1", "model": "gpt-4o",
            "choices": [{ "message": { "content": "hello" }, "finish_reason": "stop" }],
            "usage": { "prompt_tokens": 10, "completion_tokens": 5 }
        });
        let out = openai_response_to_claude(&resp);
        assert_eq!(out["type"], json!("message"));
        assert_eq!(out["content"][0]["type"], json!("text"));
        assert_eq!(out["content"][0]["text"], json!("hello"));
        assert_eq!(out["stop_reason"], json!("end_turn"));
        assert_eq!(out["usage"]["input_tokens"], json!(10));
        assert_eq!(out["usage"]["output_tokens"], json!(5));
    }

    #[test]
    fn response_length_maps_to_max_tokens() {
        let resp = json!({
            "choices": [{ "message": { "content": "x" }, "finish_reason": "length" }]
        });
        let out = openai_response_to_claude(&resp);
        assert_eq!(out["stop_reason"], json!("max_tokens"));
    }

    #[test]
    fn response_tool_calls_become_tool_use() {
        let resp = json!({
            "choices": [{
                "message": { "content": Value::Null, "tool_calls": [
                    { "id": "c1", "function": { "name": "f", "arguments": "{\"a\":1}" } }
                ]},
                "finish_reason": "tool_calls"
            }]
        });
        let out = openai_response_to_claude(&resp);
        assert_eq!(out["stop_reason"], json!("tool_use"));
        assert_eq!(out["content"][0]["type"], json!("tool_use"));
        assert_eq!(out["content"][0]["name"], json!("f"));
        assert_eq!(out["content"][0]["input"], json!({ "a": 1 }));
    }

    #[test]
    fn think_tags_split_into_thinking_blocks() {
        let blocks = split_think_tagged_text("a<think>reason</think>b");
        assert_eq!(blocks.len(), 3);
        assert_eq!(blocks[0], json!({ "type": "text", "text": "a" }));
        assert_eq!(blocks[1], json!({ "type": "thinking", "thinking": "reason" }));
        assert_eq!(blocks[2], json!({ "type": "text", "text": "b" }));
    }

    #[test]
    fn tool_choice_mappings() {
        assert_eq!(convert_tool_choice(Some(&json!({ "type": "any" }))), json!("required"));
        assert_eq!(
            convert_tool_choice(Some(&json!({ "type": "tool", "name": "f" }))),
            json!({ "type": "function", "function": { "name": "f" } })
        );
        assert_eq!(convert_tool_choice(None), json!("auto"));
    }
}
