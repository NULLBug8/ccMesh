use serde_json::Value;

use crate::modules::transform::transformer::UpstreamFormat;

/// 一次请求的真实 token 用量（含缓存创建/读取）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TokenUsage {
    pub input: i64,
    pub output: i64,
    pub cache_creation: i64,
    pub cache_read: i64,
}

/// 从非流式上游响应体解析真实 token 用量，按端点上游格式区分字段名。
/// Claude: `usage.input_tokens / output_tokens / cache_creation_input_tokens / cache_read_input_tokens`；
/// OpenAI: `usage.prompt_tokens / completion_tokens`（无缓存维度，记 0）。
pub fn from_response(body: &Value, format: UpstreamFormat) -> TokenUsage {
    let usage = body.get("usage");
    match format {
        UpstreamFormat::Claude => TokenUsage {
            input: field(usage, "input_tokens"),
            output: field(usage, "output_tokens"),
            cache_creation: field(usage, "cache_creation_input_tokens"),
            cache_read: field(usage, "cache_read_input_tokens"),
        },
        UpstreamFormat::OpenAiChat => TokenUsage {
            input: field(usage, "prompt_tokens"),
            output: field(usage, "completion_tokens"),
            cache_creation: 0,
            cache_read: 0,
        },
    }
}

fn field(usage: Option<&Value>, key: &str) -> i64 {
    usage
        .and_then(|u| u.get(key))
        .and_then(|v| v.as_i64())
        .unwrap_or(0)
}

/// 流式 SSE token 用量累积器：逐分片喂入，按行解析 `data:` JSON，结束读出 `TokenUsage`。
pub struct UsageAccumulator {
    format: UpstreamFormat,
    buf: String,
    input: i64,
    output: i64,
    cache_creation: i64,
    cache_read: i64,
}

impl UsageAccumulator {
    pub fn new(format: UpstreamFormat) -> Self {
        Self {
            format,
            buf: String::new(),
            input: 0,
            output: 0,
            cache_creation: 0,
            cache_read: 0,
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
                        if let Some(c) = u.get("cache_creation_input_tokens").and_then(|v| v.as_i64()) {
                            self.cache_creation = c;
                        }
                        if let Some(c) = u.get("cache_read_input_tokens").and_then(|v| v.as_i64()) {
                            self.cache_read = c;
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
                        if let Some(c) = u.get("cache_creation_input_tokens").and_then(|v| v.as_i64()) {
                            if c > 0 {
                                self.cache_creation = c;
                            }
                        }
                        if let Some(c) = u.get("cache_read_input_tokens").and_then(|v| v.as_i64()) {
                            if c > 0 {
                                self.cache_read = c;
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

    pub fn finish(self) -> TokenUsage {
        TokenUsage {
            input: self.input,
            output: self.output,
            cache_creation: self.cache_creation,
            cache_read: self.cache_read,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn tu(input: i64, output: i64, cache_creation: i64, cache_read: i64) -> TokenUsage {
        TokenUsage { input, output, cache_creation, cache_read }
    }

    #[test]
    fn claude_non_stream() {
        let body = json!({ "usage": { "input_tokens": 100, "output_tokens": 50 } });
        assert_eq!(from_response(&body, UpstreamFormat::Claude), tu(100, 50, 0, 0));
    }

    #[test]
    fn claude_non_stream_with_cache() {
        let body = json!({ "usage": {
            "input_tokens": 100, "output_tokens": 50,
            "cache_creation_input_tokens": 30, "cache_read_input_tokens": 70
        } });
        assert_eq!(from_response(&body, UpstreamFormat::Claude), tu(100, 50, 30, 70));
    }

    #[test]
    fn openai_non_stream() {
        let body = json!({ "usage": { "prompt_tokens": 30, "completion_tokens": 12 } });
        assert_eq!(from_response(&body, UpstreamFormat::OpenAiChat), tu(30, 12, 0, 0));
    }

    #[test]
    fn missing_usage_is_zero() {
        assert_eq!(from_response(&json!({}), UpstreamFormat::Claude), tu(0, 0, 0, 0));
    }

    #[test]
    fn claude_sse_accumulates() {
        let mut acc = UsageAccumulator::new(UpstreamFormat::Claude);
        acc.feed(b"data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":120,\"output_tokens\":0}}}\n\n");
        acc.feed(b"data: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":64}}\n\n");
        acc.feed(b"data: [DONE]\n\n");
        assert_eq!(acc.finish(), tu(120, 64, 0, 0));
    }

    #[test]
    fn claude_sse_accumulates_cache() {
        let mut acc = UsageAccumulator::new(UpstreamFormat::Claude);
        acc.feed(b"data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":10,\"output_tokens\":0,\"cache_creation_input_tokens\":40,\"cache_read_input_tokens\":80}}}\n\n");
        acc.feed(b"data: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":64}}\n\n");
        acc.feed(b"data: [DONE]\n\n");
        assert_eq!(acc.finish(), tu(10, 64, 40, 80));
    }

    #[test]
    fn openai_sse_accumulates_last_usage() {
        let mut acc = UsageAccumulator::new(UpstreamFormat::OpenAiChat);
        acc.feed(b"data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n");
        acc.feed(b"data: {\"choices\":[],\"usage\":{\"prompt_tokens\":40,\"completion_tokens\":20}}\n\n");
        acc.feed(b"data: [DONE]\n\n");
        assert_eq!(acc.finish(), tu(40, 20, 0, 0));
    }

    #[test]
    fn sse_handles_split_chunks() {
        let mut acc = UsageAccumulator::new(UpstreamFormat::OpenAiChat);
        acc.feed(b"data: {\"usage\":{\"prompt_tokens\":5,");
        acc.feed(b"\"completion_tokens\":7}}\n\n");
        assert_eq!(acc.finish(), tu(5, 7, 0, 0));
    }
}
