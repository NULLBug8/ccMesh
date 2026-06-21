//! 工具调用 JSON 规范化（移植自 cc-switch proxy/json_canonical.rs）。
//!
//! 把工具调用的 `arguments` 序列化为**稳定**的 JSON 串：递归排序对象键，
//! 提升上游 prefix-cache 命中率。空对象 `{}` 天然序列化为 `"{}"`，避免严格后端
//! （如 Minimax）以 400 `invalid function arguments json string` 拒绝空 `arguments`。
//!
//! 注：cc-switch 还有针对 Codex/Responses 路径的 `canonicalize_tool_arguments*`
//! 字符串系列辅助函数，本项目（Claude↔OpenAI Chat）无该路径，未移植。

use serde_json::Value;

/// 产出排序键的紧凑 JSON 串（对象键递归排序，数组保序）。
pub fn canonical_json_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => serde_json::to_string(value)
            .expect("serializing a JSON string for canonical output should not fail"),
        Value::Array(values) => {
            let parts = values.iter().map(canonical_json_string).collect::<Vec<_>>();
            format!("[{}]", parts.join(","))
        }
        Value::Object(map) => {
            let mut entries = map.iter().collect::<Vec<_>>();
            entries.sort_by_key(|(left, _)| *left);
            let parts = entries
                .into_iter()
                .map(|(key, value)| {
                    let key = serde_json::to_string(key).expect(
                        "serializing a JSON object key for canonical output should not fail",
                    );
                    format!("{key}:{}", canonical_json_string(value))
                })
                .collect::<Vec<_>>();
            format!("{{{}}}", parts.join(","))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sorts_nested_object_keys_stably() {
        let left = json!({ "b": 2, "a": { "d": true, "c": [3, {"z": 1, "y": 2}] } });
        let right = json!({ "a": { "c": [3, {"y": 2, "z": 1}], "d": true }, "b": 2 });
        assert_eq!(canonical_json_string(&left), canonical_json_string(&right));
        assert_eq!(
            canonical_json_string(&left),
            r#"{"a":{"c":[3,{"y":2,"z":1}],"d":true},"b":2}"#
        );
    }

    #[test]
    fn empty_object_serializes_as_empty_braces() {
        assert_eq!(canonical_json_string(&json!({})), "{}");
    }

    #[test]
    fn preserves_string_and_scalar_values() {
        assert_eq!(canonical_json_string(&json!("hi")), "\"hi\"");
        assert_eq!(canonical_json_string(&json!(42)), "42");
        assert_eq!(canonical_json_string(&json!(null)), "null");
    }
}
