use serde_json::Value;

use crate::modules::transform::transformer::UpstreamFormat;

/// 从非流式上游响应体解析真实 token 用量，按端点上游格式区分字段名。
/// Claude: `usage.input_tokens / output_tokens`；OpenAI: `usage.prompt_tokens / completion_tokens`。
pub fn from_response(body: &Value, format: UpstreamFormat) -> (i64, i64) {
    let usage = body.get("usage");
    match format {
        UpstreamFormat::Claude => (field(usage, "input_tokens"), field(usage, "output_tokens")),
        UpstreamFormat::OpenAiChat => {
            (field(usage, "prompt_tokens"), field(usage, "completion_tokens"))
        }
    }
}

fn field(usage: Option<&Value>, key: &str) -> i64 {
    usage
        .and_then(|u| u.get(key))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

/// 流式 SSE token 用量累积器：逐分片喂入，按行解析 `data:` JSON，结束读出 (input, output)。
pub struct UsageAccumulator {
    format: UpstreamFormat,
    buf: String,
    input: i64,
    output: i64,
}

impl UsageAccumulator {
    pub fn new(format: UpstreamFormat) -> Self {
        Self {
            format,
            buf: String::new(),
            input: 0,
            output: 0,
        }
    }

    /// 喂入一段响应字节（SSE 分片，可在任意位置切分）。
    pub fn feed(&mut self, chunk: &[u8]) {
        self.buf.push_str(&String::from_utf8_lossy(chunk));
        while let Some(nl) = self.buf.find('\n') {
            let line = self.buf[..nl].trim().to_string();
            self.buf.drain(..=nl);
            let Some(data) = line.strip_prefix("data:") else {
                continue;
            };
            let data = data.trim();
            if data.is_empty() || data == "[DONE]" {
                continue;
            }
            if let Ok(j) = serde_json::from_str::<Value>(data) {
                self.feed_json(&j);
            }
        }
    }

    fn feed_json(&mut self, j: &Value) {
        match self.format {
            UpstreamFormat::Claude => match j.get("type").and_then(|v| v.as_str()) {
                Some("message_start") => {
                    if let Some(u) = j.get("message").and_then(|m| m.get("usage")) {
                        if let Some(i) = u.get("input_tokens").and_then(|v| v.as_i64()) {
                            self.input = i;
                        }
                        if let Some(o) = u.get("output_tokens").and_then(|v| v.as_i64()) {
                            self.output = o;
                        }
                    }
                }
                Some("message_delta") => {
                    if let Some(u) = j.get("usage") {
                        if let Some(o) = u.get("output_tokens").and_then(|v| v.as_i64()) {
                            self.output = o;
                        }
                        // 部分 provider 在 delta 修正 input；取正值覆盖
                        if let Some(i) = u.get("input_tokens").and_then(|v| v.as_i64()) {
                            if i > 0 {
                                self.input = i;
                            }
                        }
                    }
                }
                _ => {}
            },
            UpstreamFormat::OpenAiChat => {
                if let Some(u) = j.get("usage") {
                    if !u.is_null() {
                        if let Some(i) = u.get("prompt_tokens").and_then(|v| v.as_i64()) {
                            self.input = i;
                        }
                        if let Some(o) = u.get("completion_tokens").and_then(|v| v.as_i64()) {
                            self.output = o;
                        }
                    }
                }
            }
        }
    }

    pub fn finish(self) -> (i64, i64) {
        (self.input, self.output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn claude_non_stream() {
        let body = json!({ "usage": { "input_tokens": 100, "output_tokens": 50 } });
        assert_eq!(from_response(&body, UpstreamFormat::Claude), (100, 50));
    }

    #[test]
    fn openai_non_stream() {
        let body = json!({ "usage": { "prompt_tokens": 30, "completion_tokens": 12 } });
        assert_eq!(from_response(&body, UpstreamFormat::OpenAiChat), (30, 12));
    }

    #[test]
    fn missing_usage_is_zero() {
        assert_eq!(from_response(&json!({}), UpstreamFormat::Claude), (0, 0));
    }

    #[test]
    fn claude_sse_accumulates() {
        let mut acc = UsageAccumulator::new(UpstreamFormat::Claude);
        acc.feed(b"data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":120,\"output_tokens\":0}}}\n\n");
        acc.feed(b"data: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":64}}\n\n");
        acc.feed(b"data: [DONE]\n\n");
        assert_eq!(acc.finish(), (120, 64));
    }

    #[test]
    fn openai_sse_accumulates_last_usage() {
        let mut acc = UsageAccumulator::new(UpstreamFormat::OpenAiChat);
        acc.feed(b"data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n");
        acc.feed(b"data: {\"choices\":[],\"usage\":{\"prompt_tokens\":40,\"completion_tokens\":20}}\n\n");
        acc.feed(b"data: [DONE]\n\n");
        assert_eq!(acc.finish(), (40, 20));
    }

    #[test]
    fn sse_handles_split_chunks() {
        let mut acc = UsageAccumulator::new(UpstreamFormat::OpenAiChat);
        acc.feed(b"data: {\"usage\":{\"prompt_tokens\":5,");
        acc.feed(b"\"completion_tokens\":7}}\n\n");
        assert_eq!(acc.finish(), (5, 7));
    }
}
