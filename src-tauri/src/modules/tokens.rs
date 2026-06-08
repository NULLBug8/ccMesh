use serde_json::Value;

const BASE_TOKENS: i64 = 10;
const PER_MESSAGE_OVERHEAD: i64 = 10;
const SYSTEM_OVERHEAD: i64 = 5;

/// 文本 token 近似：按前 500 字符的中文占比动态调整 chars/token
/// （英文≈4，纯中文≈1.5），最少 1。
pub fn estimate_text(text: &str) -> i64 {
    if text.is_empty() {
        return 0;
    }
    let sample: Vec<char> = text.chars().take(500).collect();
    let sample_len = sample.len() as f64;
    let cjk = sample
        .iter()
        .filter(|&&c| ('\u{4E00}'..='\u{9FFF}').contains(&c))
        .count() as f64;
    let ratio = if sample_len > 0.0 { cjk / sample_len } else { 0.0 };
    let chars_per_token = 4.0 - 2.5 * ratio;
    let count = text.chars().count() as f64;
    ((count / chars_per_token) as i64).max(1)
}

fn estimate_block(block: &Value) -> i64 {
    match block.get("type").and_then(|t| t.as_str()) {
        Some("text") => block
            .get("text")
            .and_then(|t| t.as_str())
            .map(estimate_text)
            .unwrap_or(0),
        Some("image") => 85, // 近似下限（精确为 width*height/750）
        Some("tool_use") => block
            .get("input")
            .map(|i| (i.to_string().len() as i64 / 4).max(1))
            .unwrap_or(10),
        Some("tool_result") => block.get("content").map(estimate_any).unwrap_or(10),
        _ => (block.to_string().len() as i64 / 4).max(1),
    }
}

fn estimate_any(v: &Value) -> i64 {
    match v {
        Value::String(s) => estimate_text(s),
        Value::Array(arr) => arr.iter().map(estimate_block).sum(),
        Value::Null => 0,
        other => (other.to_string().len() as i64 / 4).max(1),
    }
}

/// 估算输入 token：base + system(+5) + 每条 message(+10)。
pub fn estimate_input_tokens(system: Option<&Value>, messages: &Value) -> i64 {
    let mut total = BASE_TOKENS;
    if let Some(sys) = system {
        total += estimate_any(sys) + SYSTEM_OVERHEAD;
    }
    if let Some(arr) = messages.as_array() {
        for m in arr {
            total += PER_MESSAGE_OVERHEAD + m.get("content").map(estimate_any).unwrap_or(0);
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn english_text_about_four_chars_per_token() {
        assert_eq!(estimate_text("aaaaaaaa"), 2); // 8/4
    }

    #[test]
    fn chinese_text_denser() {
        // 4 个中文字符，ratio=1 → chars_per_token=1.5 → 4/1.5≈2
        assert!(estimate_text("你好世界") >= 2);
    }

    #[test]
    fn input_tokens_accumulate_overhead() {
        let messages = json!([{ "role": "user", "content": "hi" }]);
        let n = estimate_input_tokens(Some(&json!("sys")), &messages);
        assert!(n > BASE_TOKENS); // base + system + message overhead
    }
}
