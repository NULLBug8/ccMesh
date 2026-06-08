use serde::Serialize;
use serde_json::{json, Value};

use crate::error::AppResult;
use crate::modules::tokens::estimate_input_tokens;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenCount {
    pub input_tokens: i64,
}

/// 估算请求输入 token（供 `/v1/messages/count_tokens` 与前端工具使用）。
#[tauri::command]
pub fn count_tokens(request: Value) -> AppResult<TokenCount> {
    let system = request.get("system");
    let messages = request.get("messages").cloned().unwrap_or_else(|| json!([]));
    Ok(TokenCount {
        input_tokens: estimate_input_tokens(system, &messages),
    })
}
