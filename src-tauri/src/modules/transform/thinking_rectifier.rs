//! Thinking 签名整流器（移植自 cc-switch proxy/thinking_rectifier.rs）。
//!
//! 当上游（多为第三方 Claude 兼容后端）因 thinking 签名校验失败返回错误时，
//! 自动移除请求体中有问题的 thinking 块与 signature 字段后重试，对客户端透明。
//! 作用于 **Anthropic 请求体**，故对 Claude→Claude 直通路径最有效。

use serde_json::Value;

use crate::models::rules::DegradationRules;

/// 整流器配置（本项目只做签名整流，砍掉 cc-switch 的 budget/media 子开关）。
#[derive(Debug, Clone)]
pub struct RectifierConfig {
    /// 总开关。
    pub enabled: bool,
    /// thinking 签名整流子开关。
    pub request_thinking_signature: bool,
}

impl Default for RectifierConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            request_thinking_signature: true,
        }
    }
}

/// 整流结果。
impl RectifierConfig {
    pub fn from_rules(rules: &DegradationRules) -> Self {
        Self {
            enabled: rules.enabled,
            request_thinking_signature: rules.request_thinking_signature,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RectifyResult {
    /// 是否应用了整流。
    pub applied: bool,
    pub removed_thinking_blocks: usize,
    pub removed_redacted_thinking_blocks: usize,
    pub removed_signature_fields: usize,
}

/// 检测错误消息是否需要触发 thinking 签名整流。
///
/// 对错误体**原始文本小写 substring 匹配**——错误体即便是嵌套 JSON 串也能命中，
/// 无需结构化解析。
pub fn should_rectify_thinking_signature(
    error_message: Option<&str>,
    config: &RectifierConfig,
) -> bool {
    if !config.enabled || !config.request_thinking_signature {
        return false;
    }
    let Some(msg) = error_message else {
        return false;
    };
    let lower = msg.to_lowercase();

    // 场景1: thinking block 中的签名无效（"Invalid `signature` in `thinking` block"）
    if lower.contains("invalid")
        && lower.contains("signature")
        && lower.contains("thinking")
        && lower.contains("block")
    {
        return true;
    }
    // 场景1b: Gemini/第三方："Thought signature is not valid"
    if lower.contains("thought signature")
        && (lower.contains("not valid") || lower.contains("invalid"))
    {
        return true;
    }
    // 场景2: "must start with a thinking block"
    if lower.contains("must start with a thinking block") {
        return true;
    }
    // 场景3: "Expected `thinking` or `redacted_thinking`, but found `tool_use`"
    if lower.contains("expected")
        && (lower.contains("thinking") || lower.contains("redacted_thinking"))
        && lower.contains("found")
        && lower.contains("tool_use")
    {
        return true;
    }
    // 场景4: "signature: Field required"
    if lower.contains("signature") && lower.contains("field required") {
        return true;
    }
    // 场景5: "xxx.signature: Extra inputs are not permitted"
    if lower.contains("signature") && lower.contains("extra inputs are not permitted") {
        return true;
    }
    // 场景6: thinking/redacted_thinking "cannot be modified"
    if (lower.contains("thinking") || lower.contains("redacted_thinking"))
        && lower.contains("cannot be modified")
    {
        return true;
    }
    // 场景7: 非法请求兜底
    if lower.contains("非法请求")
        || lower.contains("illegal request")
        || lower.contains("invalid request")
    {
        return true;
    }

    false
}

/// 对 Anthropic 请求体做最小侵入整流（原地修改 `body`）：
/// - 移除 messages[*].content 中的 thinking / redacted_thinking 块
/// - 移除任意块上遗留的 signature 字段
/// - 特定条件下移除顶层 thinking 字段
pub fn rectify_anthropic_request(body: &mut Value) -> RectifyResult {
    let mut result = RectifyResult::default();

    let messages = match body.get_mut("messages").and_then(|m| m.as_array_mut()) {
        Some(m) => m,
        None => return result,
    };

    for msg in messages.iter_mut() {
        let content = match msg.get_mut("content").and_then(|c| c.as_array_mut()) {
            Some(c) => c,
            None => continue,
        };

        let mut new_content = Vec::with_capacity(content.len());
        let mut content_modified = false;

        for block in content.iter() {
            match block.get("type").and_then(|t| t.as_str()) {
                Some("thinking") => {
                    result.removed_thinking_blocks += 1;
                    content_modified = true;
                    continue;
                }
                Some("redacted_thinking") => {
                    result.removed_redacted_thinking_blocks += 1;
                    content_modified = true;
                    continue;
                }
                _ => {}
            }

            if block.get("signature").is_some() {
                let mut block_clone = block.clone();
                if let Some(obj) = block_clone.as_object_mut() {
                    obj.remove("signature");
                    result.removed_signature_fields += 1;
                    content_modified = true;
                    new_content.push(Value::Object(obj.clone()));
                    continue;
                }
            }

            new_content.push(block.clone());
        }

        if content_modified {
            result.applied = true;
            *content = new_content;
        }
    }

    let messages_snapshot: Vec<Value> = body
        .get("messages")
        .and_then(|m| m.as_array())
        .map(|a| a.to_vec())
        .unwrap_or_default();

    if should_remove_top_level_thinking(body, &messages_snapshot) {
        if let Some(obj) = body.as_object_mut() {
            obj.remove("thinking");
            result.applied = true;
        }
    }

    result
}

/// 判断是否需要移除顶层 thinking 字段：
/// thinking.type=="enabled" 且最后一条 assistant 消息首块非 thinking/redacted_thinking
/// 且含 tool_use。
fn should_remove_top_level_thinking(body: &Value, messages: &[Value]) -> bool {
    let thinking_enabled = body
        .get("thinking")
        .and_then(|t| t.get("type"))
        .and_then(|t| t.as_str())
        == Some("enabled");
    if !thinking_enabled {
        return false;
    }

    let last_assistant = messages
        .iter()
        .rev()
        .find(|m| m.get("role").and_then(|r| r.as_str()) == Some("assistant"));

    let last_assistant_content = match last_assistant
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
    {
        Some(c) if !c.is_empty() => c,
        _ => return false,
    };

    let first_block_type = last_assistant_content
        .first()
        .and_then(|b| b.get("type"))
        .and_then(|t| t.as_str());
    let missing_thinking_prefix =
        first_block_type != Some("thinking") && first_block_type != Some("redacted_thinking");
    if !missing_thinking_prefix {
        return false;
    }

    last_assistant_content
        .iter()
        .any(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_use"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn enabled() -> RectifierConfig {
        RectifierConfig::default()
    }

    #[test]
    fn detect_invalid_signature_with_and_without_backticks() {
        assert!(should_rectify_thinking_signature(
            Some("messages.1.content.0: Invalid `signature` in `thinking` block"),
            &enabled()
        ));
        assert!(should_rectify_thinking_signature(
            Some("Messages.1.Content.0: invalid signature in thinking block"),
            &enabled()
        ));
    }

    #[test]
    fn detect_nested_json_error_body() {
        let nested = r#"{"error":{"message":"{\"type\":\"error\",\"error\":{\"message\":\"***.content.0: Invalid `signature` in `thinking` block\"}}"}}"#;
        assert!(should_rectify_thinking_signature(Some(nested), &enabled()));
    }

    #[test]
    fn detect_thought_signature_and_expected_found_tool_use() {
        assert!(should_rectify_thinking_signature(
            Some("Unable to submit request because Thought signature is not valid."),
            &enabled()
        ));
        assert!(should_rectify_thinking_signature(
            Some("Expected `thinking` or `redacted_thinking`, but found `tool_use`"),
            &enabled()
        ));
    }

    #[test]
    fn detect_field_required_extra_inputs_cannot_be_modified() {
        assert!(should_rectify_thinking_signature(
            Some("signature: Field required"),
            &enabled()
        ));
        assert!(should_rectify_thinking_signature(
            Some("messages.0.content.0.signature: Extra inputs are not permitted"),
            &enabled()
        ));
        assert!(should_rectify_thinking_signature(
            Some("thinking or redacted_thinking blocks cannot be modified"),
            &enabled()
        ));
    }

    #[test]
    fn config_off_never_matches() {
        let off = RectifierConfig {
            enabled: true,
            request_thinking_signature: false,
        };
        assert!(!should_rectify_thinking_signature(
            Some("Invalid `signature` in `thinking` block"),
            &off
        ));
        let master_off = RectifierConfig {
            enabled: false,
            request_thinking_signature: true,
        };
        assert!(!should_rectify_thinking_signature(
            Some("Invalid `signature` in `thinking` block"),
            &master_off
        ));
        assert!(!should_rectify_thinking_signature(None, &enabled()));
    }

    #[test]
    fn rectify_removes_thinking_blocks_and_signature_fields() {
        let mut body = json!({
            "messages": [{
                "role": "assistant",
                "content": [
                    { "type": "thinking", "thinking": "x", "signature": "sig1" },
                    { "type": "redacted_thinking", "data": "y" },
                    { "type": "text", "text": "hi", "signature": "sig2" }
                ]
            }]
        });
        let r = rectify_anthropic_request(&mut body);
        assert!(r.applied);
        assert_eq!(r.removed_thinking_blocks, 1);
        assert_eq!(r.removed_redacted_thinking_blocks, 1);
        assert_eq!(r.removed_signature_fields, 1);
        let content = body["messages"][0]["content"].as_array().unwrap();
        assert_eq!(content.len(), 1);
        assert_eq!(content[0]["type"], json!("text"));
        assert!(content[0].get("signature").is_none());
    }

    #[test]
    fn rectify_removes_top_level_thinking_when_tool_use_without_prefix() {
        let mut body = json!({
            "thinking": { "type": "enabled", "budget_tokens": 1024 },
            "messages": [{
                "role": "assistant",
                "content": [
                    { "type": "text", "text": "calling" },
                    { "type": "tool_use", "id": "t1", "name": "f", "input": {} }
                ]
            }]
        });
        let r = rectify_anthropic_request(&mut body);
        assert!(r.applied);
        assert!(body.get("thinking").is_none());
    }

    #[test]
    fn rectify_noop_without_thinking_or_signature() {
        let mut body = json!({
            "messages": [{ "role": "user", "content": [{ "type": "text", "text": "hi" }] }]
        });
        let r = rectify_anthropic_request(&mut body);
        assert!(!r.applied);
    }
}
