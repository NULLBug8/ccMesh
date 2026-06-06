use serde_json::{json, Value};

use crate::modules::transform::types::{build_claude_event, map_finish_reason, StreamContext};

/// 有状态的流式转换器：逐个消费 OpenAI Chat 流 chunk，产出 Claude SSE 事件文本。
///
/// 用法：每收到一个上游 `data:` chunk JSON 调用 [`process_chunk`]，
/// 收到 `data: [DONE]` 时调用 [`finish`] 收尾。
pub struct StreamConverter {
    ctx: StreamContext,
    stop_reason: String,
    saw_finish: bool,
}

impl StreamConverter {
    pub fn new(model: String, input_tokens: i64) -> Self {
        let mut ctx = StreamContext::default();
        ctx.model_name = model;
        ctx.input_tokens = input_tokens;
        Self {
            ctx,
            stop_reason: "end_turn".to_string(),
            saw_finish: false,
        }
    }

    /// 当前累积的 (input_tokens, output_tokens)，供流结束后统计记录真实用量。
    pub fn usage(&self) -> (i64, i64) {
        (self.ctx.input_tokens, self.ctx.output_tokens)
    }

    fn message_start(&mut self, id: &Value, events: &mut Vec<String>) {
        if self.ctx.message_start_sent {
            return;
        }
        self.ctx.message_start_sent = true;
        events.push(build_claude_event(
            "message_start",
            &json!({
                "type": "message_start",
                "message": {
                    "id": id.clone(),
                    "type": "message",
                    "role": "assistant",
                    "content": [],
                    "model": self.ctx.model_name,
                    "stop_reason": Value::Null,
                    "stop_sequence": Value::Null,
                    "usage": { "input_tokens": self.ctx.input_tokens, "output_tokens": 0 }
                }
            }),
        ));
    }

    fn ensure_thinking_block(&mut self, events: &mut Vec<String>) {
        if self.ctx.thinking_block_started {
            return;
        }
        self.ctx.thinking_block_started = true;
        events.push(build_claude_event(
            "content_block_start",
            &json!({
                "type": "content_block_start",
                "index": self.ctx.block_index,
                "content_block": { "type": "thinking", "thinking": "" }
            }),
        ));
    }

    fn ensure_text_block(&mut self, events: &mut Vec<String>) {
        if self.ctx.text_block_started {
            return;
        }
        self.ctx.text_block_started = true;
        events.push(build_claude_event(
            "content_block_start",
            &json!({
                "type": "content_block_start",
                "index": self.ctx.block_index,
                "content_block": { "type": "text", "text": "" }
            }),
        ));
    }

    fn close_block_if<F: Fn(&StreamContext) -> bool>(
        &mut self,
        is_open: F,
        clear: fn(&mut StreamContext),
        events: &mut Vec<String>,
    ) {
        if is_open(&self.ctx) {
            events.push(build_claude_event(
                "content_block_stop",
                &json!({ "type": "content_block_stop", "index": self.ctx.block_index }),
            ));
            clear(&mut self.ctx);
            self.ctx.block_index += 1;
        }
    }

    fn close_thinking(&mut self, events: &mut Vec<String>) {
        self.close_block_if(
            |c| c.thinking_block_started,
            |c| c.thinking_block_started = false,
            events,
        );
    }
    fn close_text(&mut self, events: &mut Vec<String>) {
        self.close_block_if(|c| c.text_block_started, |c| c.text_block_started = false, events);
    }
    fn close_tool(&mut self, events: &mut Vec<String>) {
        self.close_block_if(|c| c.tool_block_started, |c| c.tool_block_started = false, events);
    }

    fn close_open_blocks(&mut self, events: &mut Vec<String>) {
        self.close_text(events);
        self.close_thinking(events);
        self.close_tool(events);
    }

    fn handle_tool_call(&mut self, tc: &Value, events: &mut Vec<String>) {
        let id = tc.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let func = tc.get("function");
        if !id.is_empty() {
            self.close_text(events);
            self.close_thinking(events);
            self.close_tool(events);
            let name = func
                .and_then(|f| f.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            self.ctx.current_tool_id = id.to_string();
            self.ctx.current_tool_name = name.to_string();
            self.ctx.tool_arguments.clear();
            self.ctx.tool_block_started = true;
            events.push(build_claude_event(
                "content_block_start",
                &json!({
                    "type": "content_block_start",
                    "index": self.ctx.block_index,
                    "content_block": { "type": "tool_use", "id": id, "name": name, "input": {} }
                }),
            ));
        }
        if let Some(args) = func
            .and_then(|f| f.get("arguments"))
            .and_then(|v| v.as_str())
        {
            if !args.is_empty() && self.ctx.tool_block_started {
                self.ctx.tool_arguments.push_str(args);
                events.push(build_claude_event(
                    "content_block_delta",
                    &json!({
                        "type": "content_block_delta",
                        "index": self.ctx.block_index,
                        "delta": { "type": "input_json_delta", "partial_json": args }
                    }),
                ));
            }
        }
    }

    /// 处理一个 OpenAI chunk JSON，返回应发出的 Claude SSE 事件文本。
    pub fn process_chunk(&mut self, chunk: &Value) -> Vec<String> {
        let mut events = Vec::new();
        let id = chunk.get("id").cloned().unwrap_or(json!(""));
        self.message_start(&id, &mut events);

        if let Some(usage) = chunk.get("usage") {
            if !usage.is_null() {
                if let Some(it) = usage.get("prompt_tokens").and_then(|v| v.as_i64()) {
                    self.ctx.input_tokens = it;
                }
                if let Some(ot) = usage.get("completion_tokens").and_then(|v| v.as_i64()) {
                    self.ctx.output_tokens = ot;
                }
            }
        }

        let choice = chunk
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first());

        if let Some(ch) = choice {
            let delta = ch.get("delta");

            if let Some(reason) = delta
                .and_then(|d| d.get("reasoning_content"))
                .and_then(|v| v.as_str())
            {
                if !reason.is_empty() {
                    self.ensure_thinking_block(&mut events);
                    events.push(build_claude_event(
                        "content_block_delta",
                        &json!({
                            "type": "content_block_delta",
                            "index": self.ctx.block_index,
                            "delta": { "type": "thinking_delta", "thinking": reason }
                        }),
                    ));
                }
            }

            if let Some(text) = delta.and_then(|d| d.get("content")).and_then(|v| v.as_str()) {
                if !text.is_empty() {
                    self.close_thinking(&mut events);
                    self.ensure_text_block(&mut events);
                    events.push(build_claude_event(
                        "content_block_delta",
                        &json!({
                            "type": "content_block_delta",
                            "index": self.ctx.block_index,
                            "delta": { "type": "text_delta", "text": text }
                        }),
                    ));
                }
            }

            if let Some(tcs) = delta
                .and_then(|d| d.get("tool_calls"))
                .and_then(|v| v.as_array())
            {
                for tc in tcs {
                    self.handle_tool_call(tc, &mut events);
                }
            }

            if let Some(fr) = ch.get("finish_reason").and_then(|v| v.as_str()) {
                self.stop_reason =
                    map_finish_reason(Some(fr), self.ctx.tool_block_started).to_string();
                self.close_open_blocks(&mut events);
                self.saw_finish = true;
            }
        }

        events
    }

    /// 收到 `data: [DONE]` 时调用：收尾未关闭块、发 message_delta 与 message_stop。
    pub fn finish(&mut self) -> Vec<String> {
        let mut events = Vec::new();
        if !self.ctx.message_start_sent {
            self.message_start(&json!(""), &mut events);
        }
        self.close_open_blocks(&mut events);
        if !self.ctx.message_stop_sent {
            events.push(build_claude_event(
                "message_delta",
                &json!({
                    "type": "message_delta",
                    "delta": { "stop_reason": self.stop_reason, "stop_sequence": Value::Null },
                    "usage": { "output_tokens": self.ctx.output_tokens }
                }),
            ));
            events.push(build_claude_event(
                "message_stop",
                &json!({ "type": "message_stop" }),
            ));
            self.ctx.message_stop_sent = true;
        }
        let _ = self.saw_finish;
        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn join(events: Vec<String>) -> String {
        events.join("")
    }

    #[test]
    fn text_stream_emits_message_start_then_text_then_stop() {
        let mut c = StreamConverter::new("gpt-4o".into(), 5);
        let e1 = join(c.process_chunk(&json!({
            "id": "x", "choices": [{ "delta": { "content": "Hi" }, "finish_reason": Value::Null }]
        })));
        assert!(e1.contains("event: message_start"));
        assert!(e1.contains("event: content_block_start"));
        assert!(e1.contains("\"type\":\"text\""));
        assert!(e1.contains("event: content_block_delta"));
        assert!(e1.contains("\"text_delta\""));

        let e2 = join(c.process_chunk(&json!({
            "choices": [{ "delta": {}, "finish_reason": "stop" }]
        })));
        assert!(e2.contains("event: content_block_stop"));

        let done = join(c.finish());
        assert!(done.contains("event: message_delta"));
        assert!(done.contains("\"stop_reason\":\"end_turn\""));
        assert!(done.contains("event: message_stop"));
    }

    #[test]
    fn tool_stream_emits_tool_use_block_and_input_json_delta() {
        let mut c = StreamConverter::new("gpt-4o".into(), 0);
        c.process_chunk(&json!({ "id": "x", "choices": [{ "delta": { "content": "" } }] }));
        let e = join(c.process_chunk(&json!({
            "choices": [{ "delta": { "tool_calls": [
                { "id": "t1", "function": { "name": "f", "arguments": "{\"a\":" } }
            ]}}]
        })));
        assert!(e.contains("\"type\":\"tool_use\""));
        assert!(e.contains("\"input_json_delta\""));
        assert!(e.contains("\"partial_json\":\"{\\\"a\\\":\""));

        let fin = join(c.process_chunk(&json!({
            "choices": [{ "delta": {}, "finish_reason": "tool_calls" }]
        })));
        assert!(fin.contains("event: content_block_stop"));
        let done = join(c.finish());
        assert!(done.contains("\"stop_reason\":\"tool_use\""));
    }

    #[test]
    fn usage_chunk_updates_output_tokens() {
        let mut c = StreamConverter::new("m".into(), 3);
        c.process_chunk(&json!({ "id": "x", "choices": [{ "delta": { "content": "y" } }] }));
        c.process_chunk(&json!({ "choices": [], "usage": { "prompt_tokens": 3, "completion_tokens": 7 } }));
        let done = join(c.finish());
        assert!(done.contains("\"output_tokens\":7"));
    }
}
