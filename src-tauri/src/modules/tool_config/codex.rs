//! Codex 配置：操作字段 ↔ (auth.json + config.toml) 的纯逻辑。
//!
//! 字段契约：
//! - api_key      → auth.json 的 OPENAI_API_KEY（不在此处理，见 mod.rs 应用）
//! - base_url     → config.toml `[model_providers.<active>].base_url`
//! - model        → config.toml `model`
//! - review_model → config.toml `review_model`
//!
//! TOML 用 `toml_edit` 做字段级更新，**保留注释与未触及字段**（D5 决策）。

use serde_json::Value;
use toml_edit::{value, DocumentMut, Item, Table};

use crate::error::{AppError, AppResult};
use crate::models::tool_config::CodexOperationFields;

const DEFAULT_PROVIDER: &str = "OpenAI";

/// 把 config.toml 文本解析为 JSON（仅用于前端展示/检查）。解析失败返回空对象。
pub fn toml_to_json(text: &str) -> Value {
    if text.trim().is_empty() {
        return Value::Object(Default::default());
    }
    text.parse::<toml::Value>()
        .ok()
        .and_then(|t| serde_json::to_value(t).ok())
        .unwrap_or_else(|| Value::Object(Default::default()))
}

/// 从 auth.json 与 config.toml 读取操作字段（用于初始化表单）。
pub fn parse_operation_fields(auth: &Value, config_toml: &str) -> CodexOperationFields {
    let api_key = auth
        .get("OPENAI_API_KEY")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let mut f = CodexOperationFields {
        api_key,
        ..Default::default()
    };
    if let Ok(tbl) = config_toml.parse::<toml::Table>() {
        f.model = tbl
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        f.review_model = tbl
            .get("review_model")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let active = tbl
            .get("model_provider")
            .and_then(|v| v.as_str())
            .unwrap_or(DEFAULT_PROVIDER);
        f.base_url = tbl
            .get("model_providers")
            .and_then(|v| v.get(active))
            .and_then(|v| v.get("base_url"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
    }
    f
}

fn apply_fields_doc(doc: &mut DocumentMut, f: &CodexOperationFields) {
    if !f.model.is_empty() {
        doc["model"] = value(f.model.as_str());
    }
    if !f.review_model.is_empty() {
        doc["review_model"] = value(f.review_model.as_str());
    }
    if !f.base_url.is_empty() {
        let active = doc
            .get("model_provider")
            .and_then(|i| i.as_str())
            .unwrap_or(DEFAULT_PROVIDER)
            .to_string();
        let providers = doc
            .entry("model_providers")
            .or_insert(Item::Table(Table::new()));
        if let Some(ptbl) = providers.as_table_mut() {
            let prov = ptbl.entry(&active).or_insert(Item::Table(Table::new()));
            if let Some(prov_tbl) = prov.as_table_mut() {
                prov_tbl["base_url"] = value(f.base_url.as_str());
            }
        }
    }
}

/// Goal mode 开关：开启写 `[features].goals = true`，关闭删除该键（features 空则删表）。
/// 与 cc-switch `setCodexGoalMode` 一致。
fn apply_goal_mode_doc(doc: &mut DocumentMut, enabled: bool) {
    if enabled {
        let features = doc.entry("features").or_insert(Item::Table(Table::new()));
        if let Some(t) = features.as_table_mut() {
            t["goals"] = value(true);
        }
    } else {
        let mut drop_features = false;
        if let Some(features) = doc.get_mut("features").and_then(|i| i.as_table_mut()) {
            features.remove("goals");
            drop_features = features.is_empty();
        }
        if drop_features {
            doc.remove("features");
        }
    }
}

/// 整合：操作字段 + 可选开关（目前 goal_mode）写进 config.toml，保留注释/其它键。
pub fn build_codex_config(
    config_toml: &str,
    f: &CodexOperationFields,
    goal_mode: Option<bool>,
) -> AppResult<String> {
    let mut doc: DocumentMut = config_toml
        .parse()
        .map_err(|e| AppError::InvalidArgument(format!("config.toml 解析失败: {e}")))?;
    apply_fields_doc(&mut doc, f);
    if let Some(g) = goal_mode {
        apply_goal_mode_doc(&mut doc, g);
    }
    Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SAMPLE: &str = r#"# 顶部注释保留
model_provider = "OpenAI"
model = "gpt-5.5"
review_model = "gpt-5.5"
model_reasoning_effort = "high"   # 行内注释保留

[model_providers.OpenAI]
requires_openai_auth = true
wire_api = "responses"
base_url = "http://127.0.0.1:3000/v1"
name = "OpenAI"
"#;

    #[test]
    fn parse_reads_fields() {
        let auth = json!({ "OPENAI_API_KEY": "sk-x" });
        let f = parse_operation_fields(&auth, SAMPLE);
        assert_eq!(f.api_key, "sk-x");
        assert_eq!(f.model, "gpt-5.5");
        assert_eq!(f.review_model, "gpt-5.5");
        assert_eq!(f.base_url, "http://127.0.0.1:3000/v1");
    }

    #[test]
    fn apply_updates_fields_and_preserves_comments() {
        let f = CodexOperationFields {
            api_key: "ignored-here".into(),
            base_url: "http://127.0.0.1:8080/v1".into(),
            model: "gpt-6".into(),
            review_model: "gpt-6-mini".into(),
        };
        let out = build_codex_config(SAMPLE, &f, None).unwrap();
        assert!(out.contains("# 顶部注释保留"), "顶部注释丢失: {out}");
        assert!(out.contains("# 行内注释保留"), "行内注释丢失");
        assert!(out.contains("model = \"gpt-6\""));
        assert!(out.contains("review_model = \"gpt-6-mini\""));
        assert!(out.contains("base_url = \"http://127.0.0.1:8080/v1\""));
        // 未触及字段保留
        assert!(out.contains("wire_api = \"responses\""));
        assert!(out.contains("requires_openai_auth = true"));
        // round-trip 仍可被读回
        let back = parse_operation_fields(&json!({}), &out);
        assert_eq!(back.model, "gpt-6");
        assert_eq!(back.base_url, "http://127.0.0.1:8080/v1");
    }

    #[test]
    fn apply_empty_fields_keeps_template() {
        let f = CodexOperationFields::default();
        let out = build_codex_config(SAMPLE, &f, None).unwrap();
        assert!(out.contains("model = \"gpt-5.5\""));
        assert!(out.contains("base_url = \"http://127.0.0.1:3000/v1\""));
    }

    #[test]
    fn build_applies_goal_mode_and_preserves_comments() {
        let f = CodexOperationFields::default();
        // 开启 goal mode
        let on = build_codex_config(SAMPLE, &f, Some(true)).unwrap();
        assert!(on.contains("# 顶部注释保留"), "注释丢失: {on}");
        assert!(on.contains("[features]"));
        assert!(on.contains("goals = true"));
        // 关闭 goal mode → 删除 goals 与空 [features]
        let off = build_codex_config(&on, &f, Some(false)).unwrap();
        assert!(!off.contains("goals = true"));
        assert!(!off.contains("[features]"));
        assert!(off.contains("# 顶部注释保留"), "关闭后注释仍应保留");
        // goal_mode = None → 不动 features
        let none = build_codex_config(SAMPLE, &f, None).unwrap();
        assert!(!none.contains("[features]"));
    }

    #[test]
    fn toml_to_json_works() {
        let v = toml_to_json(SAMPLE);
        assert_eq!(v.get("model").unwrap(), "gpt-5.5");
        assert_eq!(
            v.get("model_providers")
                .unwrap()
                .get("OpenAI")
                .unwrap()
                .get("wire_api")
                .unwrap(),
            "responses"
        );
    }
}
