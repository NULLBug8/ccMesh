use axum::body::Bytes;
use axum::http::{HeaderMap, Method};
use serde_json::Value;

use crate::models::stats::{RequestTraceHeader, RequestTraceStage};

const TRACE_BODY_LIMIT_BYTES: usize = 8 * 1024;

#[derive(Default)]
pub struct TraceBodyCapture {
    bytes: Vec<u8>,
    truncated: bool,
}

impl TraceBodyCapture {
    pub fn push(&mut self, chunk: &Bytes) {
        if self.bytes.len() >= TRACE_BODY_LIMIT_BYTES {
            self.truncated = true;
            return;
        }

        let remaining = TRACE_BODY_LIMIT_BYTES - self.bytes.len();
        let take = remaining.min(chunk.len());
        self.bytes.extend_from_slice(&chunk[..take]);
        if take < chunk.len() {
            self.truncated = true;
        }
    }

    pub fn finish(self) -> Option<String> {
        finalize_body_preview(&self.bytes, self.truncated)
    }
}

pub fn request_stage(
    method: &Method,
    url: String,
    headers: &HeaderMap,
    body: &Bytes,
) -> RequestTraceStage {
    RequestTraceStage {
        method: Some(method.to_string()),
        url: Some(url),
        status_code: None,
        headers: capture_headers(headers),
        body: capture_body(body),
    }
}

pub fn request_stage_from_pairs(
    method: &Method,
    url: String,
    headers: &[(String, String)],
    body: &Bytes,
) -> RequestTraceStage {
    RequestTraceStage {
        method: Some(method.to_string()),
        url: Some(url),
        status_code: None,
        headers: capture_header_pairs(headers),
        body: capture_body(body),
    }
}

pub fn response_stage(
    url: String,
    status_code: Option<i64>,
    headers: Vec<RequestTraceHeader>,
    body: Option<String>,
) -> RequestTraceStage {
    RequestTraceStage {
        method: None,
        url: Some(url),
        status_code,
        headers,
        body,
    }
}

pub fn capture_headers(headers: &HeaderMap) -> Vec<RequestTraceHeader> {
    headers
        .iter()
        .filter_map(|(key, value)| {
            value.to_str().ok().map(|value| RequestTraceHeader {
                key: key.as_str().to_string(),
                value: redact_header_value(key.as_str(), value),
            })
        })
        .collect()
}

pub fn capture_header_pairs(headers: &[(String, String)]) -> Vec<RequestTraceHeader> {
    headers
        .iter()
        .map(|(key, value)| RequestTraceHeader {
            key: key.clone(),
            value: redact_header_value(key, value),
        })
        .collect()
}

pub fn capture_body(body: &Bytes) -> Option<String> {
    finalize_body_preview(body, false)
}

pub fn capture_text_body(text: &str) -> Option<String> {
    finalize_text_preview(text.to_string(), false)
}

pub fn json_headers() -> Vec<RequestTraceHeader> {
    vec![RequestTraceHeader {
        key: "content-type".to_string(),
        value: "application/json".to_string(),
    }]
}

pub fn sse_headers() -> Vec<RequestTraceHeader> {
    vec![RequestTraceHeader {
        key: "content-type".to_string(),
        value: "text/event-stream".to_string(),
    }]
}

fn finalize_body_preview(bytes: &[u8], already_truncated: bool) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let truncated = already_truncated || bytes.len() >= TRACE_BODY_LIMIT_BYTES;
    let text = if bytes.len() <= TRACE_BODY_LIMIT_BYTES {
        if let Ok(json) = serde_json::from_slice::<Value>(bytes) {
            serde_json::to_string_pretty(&json).unwrap_or_else(|_| String::from_utf8_lossy(bytes).into_owned())
        } else {
            String::from_utf8_lossy(bytes).into_owned()
        }
    } else {
        String::from_utf8_lossy(&bytes[..TRACE_BODY_LIMIT_BYTES]).into_owned()
    };
    finalize_text_preview(text, truncated)
}

fn finalize_text_preview(mut text: String, truncated: bool) -> Option<String> {
    if text.is_empty() {
        return None;
    }

    if text.len() > TRACE_BODY_LIMIT_BYTES {
        let mut cut = TRACE_BODY_LIMIT_BYTES;
        while cut > 0 && !text.is_char_boundary(cut) {
            cut -= 1;
        }
        text.truncate(cut);
        text.push_str("... [truncated]");
        return Some(text);
    }

    if truncated {
        text.push_str("... [truncated]");
    }

    Some(text)
}

fn redact_header_value(key: &str, value: &str) -> String {
    match key.to_ascii_lowercase().as_str() {
        "authorization" | "x-api-key" | "cookie" | "set-cookie" | "proxy-authorization" => {
            "[redacted]".to_string()
        }
        _ => value.to_string(),
    }
}
