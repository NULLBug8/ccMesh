use std::time::{Duration, Instant};

use crate::runtime::{AppHandle, State};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::models::endpoint::{
    BalanceQueryConfig, CreateEndpointRequest, Endpoint, UpdateEndpointRequest,
};
use crate::models::stats::{RequestTrace, RequestTraceHeader, RequestTraceStage};
use crate::modules::models_probe::ProbeAuth;
use crate::modules::proxy::client::{build_client, should_use_proxy};
use crate::modules::proxy::trace_capture;
#[cfg(test)]
use crate::modules::proxy::diagnostics::diagnose_upstream_error;
use crate::modules::stats::aggregator::RequestRecord;
use crate::modules::storage::{config_repo, endpoint_repo};
use crate::modules::transform::transformer::UpstreamFormat;
use crate::modules::usage::TokenUsage;
use crate::state::AppState;

const ENDPOINT_TEST_MAX_ATTEMPTS: usize = 5;
const ENDPOINT_TEST_QUICK_ATTEMPTS: usize = 1;

/// 端点配置/测试状态变更事件（payload 为空，前端收到后全量重拉相关查询）。
const ENDPOINTS_CHANGED_EVENT: &str = "endpoints-changed";

fn emit_endpoints_changed(app: &AppHandle) {
    let _ = app.emit(ENDPOINTS_CHANGED_EVENT, ());
    crate::modules::web_admin::bridge::emit(ENDPOINTS_CHANGED_EVENT, &());
}

pub fn list_endpoints(state: State<AppState>) -> AppResult<Vec<Endpoint>> {
    let conn = state.db_pool.get()?;
    endpoint_repo::list_all(&conn)
}

pub fn create_endpoint(state: State<AppState>, req: CreateEndpointRequest) -> AppResult<Endpoint> {
    let conn = state.db_pool.get()?;
    endpoint_repo::create(&conn, &req)
}

pub fn update_endpoint(
    app: AppHandle,
    state: State<AppState>,
    id: i64,
    req: UpdateEndpointRequest,
) -> AppResult<Endpoint> {
    let conn = state.db_pool.get()?;
    let ep = endpoint_repo::update(&conn, id, &req)?;
    emit_endpoints_changed(&app);
    Ok(ep)
}

pub fn delete_endpoint(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db_pool.get()?;
    endpoint_repo::delete(&conn, id)
}

pub fn reorder_endpoints(
    app: AppHandle,
    state: State<AppState>,
    ordered_ids: Vec<i64>,
) -> AppResult<()> {
    let mut conn = state.db_pool.get()?;
    endpoint_repo::reorder(&mut conn, &ordered_ids)?;
    emit_endpoints_changed(&app);
    Ok(())
}

/// 克隆端点：名称自动加 `(副本)` 后缀并避免冲突。
pub fn clone_endpoint(state: State<AppState>, id: i64) -> AppResult<Endpoint> {
    let conn = state.db_pool.get()?;
    let src = endpoint_repo::get_by_id(&conn, id)?
        .ok_or_else(|| AppError::NotFound(format!("端点 #{id} 不存在")))?;
    let base = extract_base_name(&src.name);
    let name = unique_clone_name(&conn, &base)?;
    let req = CreateEndpointRequest {
        name,
        api_url: src.api_url,
        api_key: src.api_key,
        auth_mode: src.auth_mode,
        enabled: src.enabled,
        use_proxy: src.use_proxy,
        transformer: src.transformer,
        model: src.model,
        models: src.models,
        active_models: src.active_models,
        model_mappings: src.model_mappings,
        balance_query: src.balance_query,
        remark: src.remark,
    };
    endpoint_repo::create(&conn, &req)
}

fn extract_base_name(name: &str) -> String {
    let n = name.trim();
    for marker in ["(副本)", "(Copy)"] {
        if let Some(pos) = n.rfind(marker) {
            let rest = n[pos + marker.len()..].trim();
            if rest.is_empty() || rest.chars().all(|c| c.is_ascii_digit()) {
                return n[..pos].trim().to_string();
            }
        }
    }
    n.to_string()
}

fn unique_clone_name(conn: &rusqlite::Connection, base: &str) -> AppResult<String> {
    let first = format!("{base}(副本)");
    if endpoint_repo::get_by_name(conn, &first)?.is_none() {
        return Ok(first);
    }
    let mut i = 1;
    loop {
        let cand = format!("{base}(副本) {i}");
        if endpoint_repo::get_by_name(conn, &cand)?.is_none() {
            return Ok(cand);
        }
        i += 1;
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    pub success: bool,
    pub status: String, // available / unavailable
    pub latency_ms: u64,
    pub message: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceQueryResult {
    pub success: bool,
    pub status: u16,
    pub latency_ms: u64,
    pub balance: Option<String>,
    pub currency: Option<String>,
    pub used: Option<String>,
    pub expires_at: Option<String>,
    pub limits: Vec<BalanceLimitResult>,
    pub message: String,
    pub raw: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceLimitResult {
    pub label: String,
    pub balance: Option<String>,
    pub used: Option<String>,
    pub expires_at: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceProbeTemplateResult {
    pub template_id: String,
    pub path: String,
    pub success: bool,
    pub url_reachable: bool,
    pub status_code: Option<u16>,
    pub latency_ms: u64,
    pub message: String,
    pub sample: Option<String>,
    pub config: Option<BalanceQueryConfig>,
    pub balance: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceProbeResult {
    pub status: String,
    pub results: Vec<BalanceProbeTemplateResult>,
    pub matched: Option<BalanceProbeTemplateResult>,
    pub usable_samples: Vec<BalanceProbeTemplateResult>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceTemplateAiSample {
    pub template_id: String,
    pub path: String,
    pub status_code: Option<u16>,
    pub sample: Option<String>,
}

fn render_balance_template(input: &str, ep: &Endpoint) -> String {
    input
        .replace("{{apiKey}}", &ep.api_key)
        .replace("{{apiUrl}}", ep.api_url.trim_end_matches('/'))
        .replace("{{endpointName}}", &ep.name)
}

fn json_path_value(json: &serde_json::Value, path: &str) -> Option<String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return None;
    }
    for operator in [" - ", " + ", " * ", " / "] {
        if let Some((left, right)) = trimmed.split_once(operator) {
            let left = json_path_number_operand(json, left)?;
            let right = json_path_number_operand(json, right)?;
            let value = match operator {
                " - " => left - right,
                " + " => left + right,
                " * " => left * right,
                " / " => left / right,
                _ => return None,
            };
            return Some(format_decimal(value));
        }
    }
    let mut current = json;
    let path = trimmed
        .strip_prefix("$.")
        .or_else(|| trimmed.strip_prefix('$'))?;
    if path.is_empty() {
        return Some(current.to_string());
    }
    for segment in path.split('.') {
        let key = segment.trim();
        if key.is_empty() {
            return None;
        }
        current = json_path_segment_value(current, key)?;
    }
    match current {
        serde_json::Value::Null => None,
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        serde_json::Value::Bool(value) => Some(value.to_string()),
        other => Some(other.to_string()),
    }
}

fn balance_url(ep: &Endpoint, cfg: &BalanceQueryConfig) -> String {
    let path = render_balance_template(&cfg.path, ep);
    if path.starts_with("http://") || path.starts_with("https://") {
        path
    } else {
        format!("{}{}", ep.api_url.trim_end_matches('/'), path)
    }
}

fn balance_query_presets() -> Vec<BalanceQueryConfig> {
    vec![
        BalanceQueryConfig {
            enabled: true,
            template_id: "openai".into(),
            method: "GET".into(),
            path: "/dashboard/billing/credit_grants".into(),
            headers: vec![],
            body: String::new(),
            extraction: crate::models::endpoint::BalanceExtraction {
                balance_path: "$.total_available".into(),
                currency_path: "$.currency".into(),
                used_path: "$.total_used".into(),
                expires_at_path: "$.expires_at".into(),
                limits: Vec::new(),
            },
        },
        BalanceQueryConfig {
            enabled: true,
            template_id: "openai-usage".into(),
            method: "GET".into(),
            path: "/dashboard/billing/usage".into(),
            headers: vec![crate::models::endpoint::BalanceHeader {
                name: "Authorization".into(),
                value: "Bearer {{apiKey}}".into(),
            }],
            body: String::new(),
            extraction: crate::models::endpoint::BalanceExtraction {
                balance_path: String::new(),
                currency_path: String::new(),
                used_path: "$.total_usage".into(),
                expires_at_path: String::new(),
                limits: Vec::new(),
            },
        },
        BalanceQueryConfig {
            enabled: true,
            template_id: "apimart".into(),
            method: "GET".into(),
            path: "/v1/user/balance".into(),
            headers: vec![crate::models::endpoint::BalanceHeader {
                name: "Authorization".into(),
                value: "Bearer {{apiKey}}".into(),
            }],
            body: String::new(),
            extraction: apimart_balance_extraction(),
        },
        BalanceQueryConfig {
            enabled: true,
            template_id: "apimart-legacy".into(),
            method: "GET".into(),
            path: "/user/balance".into(),
            headers: vec![crate::models::endpoint::BalanceHeader {
                name: "Authorization".into(),
                value: "Bearer {{apiKey}}".into(),
            }],
            body: String::new(),
            extraction: apimart_balance_extraction(),
        },
        BalanceQueryConfig {
            enabled: true,
            template_id: "newapi".into(),
            method: "GET".into(),
            path: "/api/user/self".into(),
            headers: vec![crate::models::endpoint::BalanceHeader {
                name: "Authorization".into(),
                value: "Bearer {{apiKey}}".into(),
            }],
            body: String::new(),
            extraction: crate::models::endpoint::BalanceExtraction {
                balance_path: "$.data.quota".into(),
                currency_path: "$.data.currency".into(),
                used_path: "$.data.used_quota".into(),
                expires_at_path: String::new(),
                limits: Vec::new(),
            },
        },
        BalanceQueryConfig {
            enabled: true,
            template_id: "one-api".into(),
            method: "GET".into(),
            path: "/api/user/self".into(),
            headers: vec![crate::models::endpoint::BalanceHeader {
                name: "Authorization".into(),
                value: "Bearer {{apiKey}}".into(),
            }],
            body: String::new(),
            extraction: crate::models::endpoint::BalanceExtraction {
                balance_path: "$.data.quota".into(),
                currency_path: String::new(),
                used_path: "$.data.used_quota".into(),
                expires_at_path: String::new(),
                limits: Vec::new(),
            },
        },
        BalanceQueryConfig {
            enabled: true,
            template_id: "sub2api".into(),
            method: "GET".into(),
            path: "/api/user/self".into(),
            headers: vec![crate::models::endpoint::BalanceHeader {
                name: "Authorization".into(),
                value: "Bearer {{apiKey}}".into(),
            }],
            body: String::new(),
            extraction: crate::models::endpoint::BalanceExtraction {
                balance_path: "$.data.quota".into(),
                currency_path: "$.data.currency".into(),
                used_path: "$.data.used_quota".into(),
                expires_at_path: "$.data.expired_time".into(),
                limits: Vec::new(),
            },
        },
        BalanceQueryConfig {
            enabled: true,
            template_id: "voapi".into(),
            method: "GET".into(),
            path: "/api/user/self".into(),
            headers: vec![crate::models::endpoint::BalanceHeader {
                name: "Authorization".into(),
                value: "Bearer {{apiKey}}".into(),
            }],
            body: String::new(),
            extraction: crate::models::endpoint::BalanceExtraction {
                balance_path: "$.data.quota".into(),
                currency_path: String::new(),
                used_path: "$.data.used_quota".into(),
                expires_at_path: "$.data.expired_time".into(),
                limits: Vec::new(),
            },
        },
        BalanceQueryConfig {
            enabled: true,
            template_id: "newapi-token".into(),
            method: "GET".into(),
            path: "/api/token".into(),
            headers: vec![crate::models::endpoint::BalanceHeader {
                name: "Authorization".into(),
                value: "Bearer {{apiKey}}".into(),
            }],
            body: String::new(),
            extraction: crate::models::endpoint::BalanceExtraction {
                balance_path: "$.data.quota".into(),
                currency_path: "$.data.currency".into(),
                used_path: "$.data.used_quota".into(),
                expires_at_path: String::new(),
                limits: Vec::new(),
            },
        },
        BalanceQueryConfig {
            enabled: true,
            template_id: "one-hub".into(),
            method: "GET".into(),
            path: "/api/user/self".into(),
            headers: vec![crate::models::endpoint::BalanceHeader {
                name: "Authorization".into(),
                value: "Bearer {{apiKey}}".into(),
            }],
            body: String::new(),
            extraction: crate::models::endpoint::BalanceExtraction {
                balance_path: "$.data.quota".into(),
                currency_path: "$.data.currency".into(),
                used_path: "$.data.used_quota".into(),
                expires_at_path: String::new(),
                limits: Vec::new(),
            },
        },
        BalanceQueryConfig {
            enabled: true,
            template_id: "newapi-user-key".into(),
            method: "GET".into(),
            path: "/api/user/self".into(),
            headers: vec![
                crate::models::endpoint::BalanceHeader {
                    name: "New-Api-User".into(),
                    value: "{{apiKey}}".into(),
                },
                crate::models::endpoint::BalanceHeader {
                    name: "Accept".into(),
                    value: "application/json".into(),
                },
            ],
            body: String::new(),
            extraction: crate::models::endpoint::BalanceExtraction {
                balance_path: "$.data.quota".into(),
                currency_path: "$.data.currency".into(),
                used_path: "$.data.used_quota".into(),
                expires_at_path: String::new(),
                limits: Vec::new(),
            },
        },
        BalanceQueryConfig {
            enabled: true,
            template_id: "crazyrouter".into(),
            method: "GET".into(),
            path: "/api/user/self".into(),
            headers: vec![
                crate::models::endpoint::BalanceHeader {
                    name: "Authorization".into(),
                    value: "Bearer {{apiKey}}".into(),
                },
                crate::models::endpoint::BalanceHeader {
                    name: "Accept".into(),
                    value: "application/json".into(),
                },
            ],
            body: String::new(),
            extraction: crate::models::endpoint::BalanceExtraction {
                balance_path: "$.data.quota / 500000".into(),
                currency_path: String::new(),
                used_path: "$.data.used_quota / 500000".into(),
                expires_at_path: String::new(),
                limits: Vec::new(),
            },
        },
        BalanceQueryConfig {
            enabled: true,
            template_id: "cafecode".into(),
            method: "GET".into(),
            path: "/v1/usage".into(),
            headers: vec![crate::models::endpoint::BalanceHeader {
                name: "Authorization".into(),
                value: "Bearer {{apiKey}}".into(),
            }],
            body: String::new(),
            extraction: crate::models::endpoint::BalanceExtraction {
                balance_path: "$.remaining".into(),
                currency_path: "$.unit".into(),
                used_path: "$.usage.today.actual_cost".into(),
                expires_at_path: "$.subscription.expires_at".into(),
                limits: vec![
                    crate::models::endpoint::BalanceLimitExtraction {
                        label: "今日额度".into(),
                        balance_path:
                            "$.subscription.daily_limit_usd - $.subscription.daily_usage_usd".into(),
                        used_path: "$.subscription.daily_usage_usd".into(),
                        expires_at_path: "$.subscription.expires_at".into(),
                    },
                    crate::models::endpoint::BalanceLimitExtraction {
                        label: "每周额度".into(),
                        balance_path:
                            "$.subscription.weekly_limit_usd - $.subscription.weekly_usage_usd"
                                .into(),
                        used_path: "$.subscription.weekly_usage_usd".into(),
                        expires_at_path: "$.subscription.expires_at".into(),
                    },
                    crate::models::endpoint::BalanceLimitExtraction {
                        label: "每月额度".into(),
                        balance_path:
                            "$.subscription.monthly_limit_usd - $.subscription.monthly_usage_usd"
                                .into(),
                        used_path: "$.subscription.monthly_usage_usd".into(),
                        expires_at_path: "$.subscription.expires_at".into(),
                    },
                ],
            },
        },
        BalanceQueryConfig {
            enabled: true,
            template_id: "tokenfor-me".into(),
            method: "GET".into(),
            path: "/v1/usage".into(),
            headers: vec![crate::models::endpoint::BalanceHeader {
                name: "Authorization".into(),
                value: "Bearer {{apiKey}}".into(),
            }],
            body: String::new(),
            extraction: crate::models::endpoint::BalanceExtraction {
                balance_path: "$.remaining".into(),
                currency_path: "$.unit".into(),
                used_path: "$.usage.today.actual_cost".into(),
                expires_at_path: "$.subscription.expires_at".into(),
                limits: vec![
                    crate::models::endpoint::BalanceLimitExtraction {
                        label: "今日额度".into(),
                        balance_path:
                            "$.subscription.daily_limit_usd - $.subscription.daily_usage_usd".into(),
                        used_path: "$.subscription.daily_usage_usd".into(),
                        expires_at_path: "$.subscription.expires_at".into(),
                    },
                    crate::models::endpoint::BalanceLimitExtraction {
                        label: "每周额度".into(),
                        balance_path:
                            "$.subscription.weekly_limit_usd - $.subscription.weekly_usage_usd"
                                .into(),
                        used_path: "$.subscription.weekly_usage_usd".into(),
                        expires_at_path: "$.subscription.expires_at".into(),
                    },
                ],
            },
        },
        BalanceQueryConfig {
            enabled: true,
            template_id: "laozhang".into(),
            method: "GET".into(),
            path: "/api/user/self".into(),
            headers: vec![
                crate::models::endpoint::BalanceHeader {
                    name: "Authorization".into(),
                    value: "{{apiKey}}".into(),
                },
                crate::models::endpoint::BalanceHeader {
                    name: "Accept".into(),
                    value: "application/json".into(),
                },
                crate::models::endpoint::BalanceHeader {
                    name: "Content-Type".into(),
                    value: "application/json".into(),
                },
            ],
            body: String::new(),
            extraction: crate::models::endpoint::BalanceExtraction {
                balance_path: "$.data.quota".into(),
                currency_path: String::new(),
                used_path: "$.data.used_quota".into(),
                expires_at_path: String::new(),
                limits: Vec::new(),
            },
        },
    ]
}

fn json_path_number_operand(json: &serde_json::Value, operand: &str) -> Option<f64> {
    let trimmed = operand.trim();
    if let Ok(value) = trimmed.parse::<f64>() {
        return Some(value);
    }
    json_path_value(json, trimmed)?.parse::<f64>().ok()
}

fn sanitize_json_sample(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                let lowered = key.to_ascii_lowercase();
                if lowered.contains("key")
                    || lowered.contains("token")
                    || lowered.contains("secret")
                    || lowered.contains("authorization")
                {
                    *child = serde_json::Value::String("***".into());
                } else {
                    sanitize_json_sample(child);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for child in items {
                sanitize_json_sample(child);
            }
        }
        _ => {}
    }
}

fn apimart_balance_extraction() -> crate::models::endpoint::BalanceExtraction {
    crate::models::endpoint::BalanceExtraction {
        balance_path: "$.balance_1d".into(),
        currency_path: String::new(),
        used_path: "$.used_1d".into(),
        expires_at_path: String::new(),
        limits: vec![
            crate::models::endpoint::BalanceLimitExtraction {
                label: "3小时额度".into(),
                balance_path: "$.balance_3h".into(),
                used_path: "$.used_3h".into(),
                expires_at_path: String::new(),
            },
            crate::models::endpoint::BalanceLimitExtraction {
                label: "每日额度".into(),
                balance_path: "$.balance_1d".into(),
                used_path: "$.used_1d".into(),
                expires_at_path: String::new(),
            },
        ],
    }
}

fn sanitize_sample_text(raw: &str) -> String {
    let mut sample = if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(raw) {
        sanitize_json_sample(&mut json);
        serde_json::to_string_pretty(&json).unwrap_or_else(|_| raw.to_string())
    } else {
        raw.to_string()
    };
    if sample.len() > 4000 {
        sample.truncate(4000);
        sample.push_str("\n... truncated");
    }
    sample
}

fn sanitize_balance_sample(raw: &str, ep: &Endpoint) -> String {
    let mut sample = sanitize_sample_text(raw);
    if !ep.api_key.trim().is_empty() {
        sample = sample.replace(&ep.api_key, "***");
    }
    sample
}

fn is_usable_balance_ai_sample(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lowered = trimmed.to_ascii_lowercase();
    if lowered.starts_with("<!doctype") || lowered.starts_with("<html") {
        return false;
    }
    let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) else {
        return false;
    };
    if json
        .get("success")
        .and_then(|value| value.as_bool())
        .is_some_and(|success| !success)
    {
        return false;
    }
    let rendered = json.to_string().to_ascii_lowercase();
    let auth_error_markers = [
        "unauthorized",
        "invalid access token",
        "invalid token",
        "access token",
        "login",
        "not logged in",
        "forbidden",
    ];
    if auth_error_markers
        .iter()
        .any(|marker| rendered.contains(marker))
    {
        return false;
    }
    if json.get("data").is_some_and(|value| value.is_null())
        && (json.get("msg").is_some() || json.get("message").is_some())
    {
        return false;
    }
    true
}

async fn run_balance_query(
    client: &reqwest::Client,
    ep: &Endpoint,
    cfg: &BalanceQueryConfig,
    openai_ua: Option<&str>,
    claude_ua: Option<&str>,
) -> AppResult<BalanceQueryResult> {
    let method =
        reqwest::Method::from_bytes(cfg.method.trim().as_bytes()).unwrap_or(reqwest::Method::GET);
    let url = balance_url(ep, cfg);
    let mut req = client.request(method, &url);
    let mut has_user_agent = false;
    for header in &cfg.headers {
        let name = header.name.trim();
        if name.is_empty() {
            continue;
        }
        if name.eq_ignore_ascii_case("user-agent") {
            has_user_agent = true;
        }
        req = req.header(name, render_balance_template(&header.value, ep));
    }
    if !has_user_agent {
        let format = UpstreamFormat::from_transformer_name(&ep.transformer);
        match format {
            UpstreamFormat::OpenAiChat | UpstreamFormat::OpenAiResponses => {
                if let Some(ua) = openai_ua.filter(|v| !v.trim().is_empty()) {
                    req = req.header("user-agent", ua);
                    req = req.header("originator", crate::utils::ua::CODEX_ORIGINATOR);
                }
            }
            UpstreamFormat::Claude => {
                if let Some(ua) = claude_ua.filter(|v| !v.trim().is_empty()) {
                    req = req.header("user-agent", ua);
                }
            }
        }
    }
    if !cfg.body.trim().is_empty() {
        req = req.body(render_balance_template(&cfg.body, ep));
    }

    let start = Instant::now();
    let resp = req
        .send()
        .await
        .map_err(|e| AppError::Proxy(format!("余额查询失败: {e}")))?;
    let latency_ms = start.elapsed().as_millis() as u64;
    let status = resp.status().as_u16();
    let raw = resp
        .text()
        .await
        .map_err(|e| AppError::Proxy(format!("读取余额响应失败: {e}")))?;
    let json: serde_json::Value = serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
    let extraction = &cfg.extraction;
    let balance = json_path_value(&json, &extraction.balance_path);
    let currency = json_path_value(&json, &extraction.currency_path);
    let used = json_path_value(&json, &extraction.used_path);
    let expires_at = json_path_value(&json, &extraction.expires_at_path);
    let limits: Vec<BalanceLimitResult> = extraction
        .limits
        .iter()
        .filter_map(|limit| {
            let label = limit.label.trim();
            if label.is_empty() {
                return None;
            }
            if !balance_limit_expression_has_positive_cap(&json, &limit.balance_path) {
                return None;
            }
            let balance = json_path_value(&json, &limit.balance_path);
            let used = json_path_value(&json, &limit.used_path);
            let expires_at = json_path_value(&json, &limit.expires_at_path);
            if balance.is_none() && used.is_none() && expires_at.is_none() {
                return None;
            }
            Some(BalanceLimitResult {
                label: label.to_string(),
                balance,
                used,
                expires_at,
            })
        })
        .collect();
    let success = status < 400 && (balance.is_some() || !limits.is_empty() || used.is_some());
    let message = if success {
        if balance.is_none() && limits.is_empty() && used.is_some() {
            "用量查询成功：站点未返回余额字段，仅返回已用量".to_string()
        } else {
            "余额查询成功".to_string()
        }
    } else if status >= 400 {
        format!("余额接口返回 HTTP {status}")
    } else {
        balance_missing_field_message(&json, &raw)
    };

    Ok(BalanceQueryResult {
        success,
        status,
        latency_ms,
        balance,
        currency,
        used,
        expires_at,
        limits,
        message,
        raw,
    })
}

fn balance_limit_expression_has_positive_cap(json: &serde_json::Value, path: &str) -> bool {
    let trimmed = path.trim();
    let Some((cap_operand, _)) = trimmed.split_once(" - ") else {
        return true;
    };
    json_path_number_operand(json, cap_operand).is_some_and(|cap| cap > 0.0)
}

fn balance_missing_field_message(json: &serde_json::Value, raw: &str) -> String {
    let trimmed = raw.trim();
    let lowered = trimmed.to_ascii_lowercase();
    if lowered.starts_with("<!doctype") || lowered.starts_with("<html") {
        return "余额接口返回 HTML 页面，可能是余额路径不正确、需要网页登录态或被站点风控拦截"
            .into();
    }
    if let Some(reason) = extract_json_message_field(json) {
        return format!("余额接口返回: {reason}");
    }
    "余额响应中未找到余额字段".to_string()
}

fn extract_json_message_field(json: &serde_json::Value) -> Option<String> {
    let candidates = [
        json.pointer("/error/message"),
        json.pointer("/message"),
        json.pointer("/msg"),
        json.pointer("/error"),
    ];
    for candidate in candidates.into_iter().flatten() {
        if let Some(message) = candidate.as_str() {
            let message = strip_trace_suffix(message);
            if !message.is_empty() {
                return Some(message);
            }
        }
    }
    None
}

fn should_try_balance_presets(result: &BalanceQueryResult) -> bool {
    !result.success
}

fn mark_balance_template_matched(
    mut result: BalanceQueryResult,
    template_id: &str,
) -> BalanceQueryResult {
    result.message = format!("{}（已自动匹配余额模板: {template_id}）", result.message);
    result
}

fn classify_balance_probe_results(results: Vec<BalanceProbeTemplateResult>) -> BalanceProbeResult {
    let matched = results.iter().find(|item| item.success).cloned();
    let usable_samples: Vec<_> = if matched.is_some() {
        vec![]
    } else {
        results
            .iter()
            .filter(|item| {
                item.url_reachable
                    && item
                        .sample
                        .as_deref()
                        .is_some_and(is_usable_balance_ai_sample)
            })
            .cloned()
            .map(|mut item| {
                item.sample = item.sample.as_deref().map(sanitize_sample_text);
                item
            })
            .collect()
    };
    let status = if matched.is_some() {
        "matched"
    } else if !usable_samples.is_empty() {
        "sampleAvailable"
    } else {
        "allFailed"
    };

    BalanceProbeResult {
        status: status.to_string(),
        results,
        matched,
        usable_samples,
    }
}

fn custom_probe_config(path: String) -> BalanceQueryConfig {
    BalanceQueryConfig {
        enabled: true,
        template_id: "custom-probe".into(),
        method: "GET".into(),
        path,
        headers: vec![crate::models::endpoint::BalanceHeader {
            name: "Authorization".into(),
            value: "Bearer {{apiKey}}".into(),
        }],
        body: String::new(),
        extraction: crate::models::endpoint::BalanceExtraction::default(),
    }
}

fn ai_chat_url(api_url: &str, format: UpstreamFormat) -> String {
    let base = api_url.trim_end_matches('/');
    match format {
        UpstreamFormat::OpenAiResponses => {
            if base.ends_with("/v1") {
                format!("{base}/responses")
            } else {
                format!("{base}/v1/responses")
            }
        }
        UpstreamFormat::OpenAiChat => {
            if base.ends_with("/v1") {
                format!("{base}/chat/completions")
            } else {
                format!("{base}/v1/chat/completions")
            }
        }
        UpstreamFormat::Claude => {
            if base.ends_with("/v1") {
                format!("{base}/messages")
            } else {
                format!("{base}/v1/messages")
            }
        }
    }
}

fn balance_template_prompt(target: &Endpoint, samples: &[BalanceTemplateAiSample]) -> String {
    let sample_text = samples
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            format!(
                r#"Sample #{index}
Probe template: {template_id}
Probe path: {path}
HTTP status: {status}
Sanitized response sample:
{sample}
"#,
                index = index + 1,
                template_id = sample.template_id,
                path = sample.path,
                status = sample
                    .status_code
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "unknown".into()),
                sample = sample.sample.clone().unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>()
        .join("\n---\n");
    format!(
        r#"You are configuring a relay balance query template.
You will receive every probe URL that returned data. Compare all samples and choose the most suitable URL/path for the final template.
Return only JSON matching this TypeScript shape:
{{
  "enabled": true,
  "templateId": "ai-generated",
  "method": "GET",
  "path": "/api/user/self",
  "headers": [{{"name":"Authorization","value":"Bearer {{{{apiKey}}}}"}}],
  "body": "",
  "extraction": {{
    "balancePath": "$.data.balance",
    "currencyPath": "",
    "usedPath": "",
    "expiresAtPath": "",
    "limits": [
      {{
        "label": "3小时额度",
        "balancePath": "$.data.three_hour.remaining",
        "usedPath": "$.data.three_hour.used",
        "expiresAtPath": "$.data.three_hour.reset_at"
      }}
    ]
  }}
}}

Rules:
- Do not include markdown.
- Keep API keys as {{{{apiKey}}}} placeholders.
- Pick the path from the sample that best represents balance/quota data.
- Choose JSON Paths that extract balance, currency, used amount, and expiry when present.
- If the response contains multiple quota periods, such as 3-hour, daily, weekly, monthly, or request limits, include all of them in extraction.limits.
- Use clear Chinese labels for limits when possible, for example "3小时额度", "一天额度", "1周额度".
- Leave a limit field empty only when that field is not present.

Endpoint name: {endpoint_name}
Endpoint base URL: {api_url}
Probe samples:
{sample_text}
"#,
        endpoint_name = target.name,
        api_url = target.api_url,
        sample_text = sample_text,
    )
}

fn extract_ai_text(format: UpstreamFormat, value: &serde_json::Value) -> Option<String> {
    match format {
        UpstreamFormat::OpenAiChat => value
            .get("choices")?
            .get(0)?
            .get("message")?
            .get("content")?
            .as_str()
            .map(str::to_string),
        UpstreamFormat::OpenAiResponses => {
            if let Some(text) = value.get("output_text").and_then(|v| v.as_str()) {
                return Some(text.to_string());
            }
            value
                .get("output")?
                .as_array()?
                .iter()
                .flat_map(|item| {
                    item.get("content")
                        .and_then(|v| v.as_array())
                        .into_iter()
                        .flatten()
                })
                .find_map(|content| content.get("text").and_then(|v| v.as_str()))
                .map(str::to_string)
        }
        UpstreamFormat::Claude => value
            .get("content")?
            .as_array()?
            .iter()
            .find_map(|content| content.get("text").and_then(|v| v.as_str()))
            .map(str::to_string),
    }
}

fn parse_ai_balance_config(text: &str) -> AppResult<BalanceQueryConfig> {
    let trimmed = text.trim();
    let json_text = if trimmed.starts_with("```") {
        let without_open = trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim();
        without_open.trim_end_matches("```").trim()
    } else if let (Some(start), Some(end)) = (trimmed.find('{'), trimmed.rfind('}')) {
        &trimmed[start..=end]
    } else {
        trimmed
    };
    let mut cfg: BalanceQueryConfig = serde_json::from_str(json_text)
        .map_err(|e| AppError::InvalidArgument(format!("AI 返回的余额模板 JSON 无法解析: {e}")))?;
    cfg.enabled = true;
    if cfg.template_id.trim().is_empty() {
        cfg.template_id = "ai-generated".into();
    }
    if cfg.method.trim().is_empty() {
        cfg.method = "GET".into();
    }
    if cfg.path.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "AI 返回的余额模板缺少 path".into(),
        ));
    }
    if cfg.extraction.balance_path.trim().is_empty()
        && cfg
            .extraction
            .limits
            .iter()
            .all(|limit| limit.balance_path.trim().is_empty())
    {
        return Err(AppError::InvalidArgument(
            "AI 返回的余额模板缺少 balancePath 或 limits[].balancePath".into(),
        ));
    }
    Ok(cfg)
}

fn balance_config_extracts_from_samples(
    cfg: &BalanceQueryConfig,
    samples: &[BalanceTemplateAiSample],
) -> bool {
    samples.iter().any(|sample| {
        let Some(raw) = sample.sample.as_deref() else {
            return false;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(raw) else {
            return false;
        };
        json_path_value(&json, &cfg.extraction.balance_path).is_some()
            || cfg.extraction.limits.iter().any(|limit| {
                json_path_value(&json, &limit.balance_path).is_some()
                    || json_path_value(&json, &limit.used_path).is_some()
            })
    })
}

fn infer_balance_config_from_samples(
    samples: &[BalanceTemplateAiSample],
) -> Option<BalanceQueryConfig> {
    for sample in samples {
        let Some(raw) = sample.sample.as_deref() else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(raw) else {
            continue;
        };
        if json_path_value(&json, "$.remaining").is_some()
            && json_path_value(&json, "$.subscription.daily_limit_usd").is_some()
        {
            return Some(usage_balance_config(&sample.path));
        }
        if json_path_value(&json, "$.balance_1d").is_some()
            || json_path_value(&json, "$.balance_3h").is_some()
        {
            return Some(apimart_balance_config(&sample.path));
        }
        if json_path_value(&json, "$.data.quota").is_some() {
            return Some(newapi_like_balance_config(&sample.path, "$.data.quota"));
        }
        if json_path_value(&json, "$.data.balance").is_some() {
            return Some(newapi_like_balance_config(&sample.path, "$.data.balance"));
        }
    }
    None
}

fn apimart_balance_config(path: &str) -> BalanceQueryConfig {
    BalanceQueryConfig {
        enabled: true,
        template_id: "apimart-auto".into(),
        method: "GET".into(),
        path: path.into(),
        headers: vec![crate::models::endpoint::BalanceHeader {
            name: "Authorization".into(),
            value: "Bearer {{apiKey}}".into(),
        }],
        body: String::new(),
        extraction: apimart_balance_extraction(),
    }
}

fn usage_balance_config(path: &str) -> BalanceQueryConfig {
    BalanceQueryConfig {
        enabled: true,
        template_id: "usage-auto".into(),
        method: "GET".into(),
        path: path.into(),
        headers: vec![crate::models::endpoint::BalanceHeader {
            name: "Authorization".into(),
            value: "Bearer {{apiKey}}".into(),
        }],
        body: String::new(),
        extraction: crate::models::endpoint::BalanceExtraction {
            balance_path: "$.remaining".into(),
            currency_path: "$.unit".into(),
            used_path: "$.usage.today.actual_cost".into(),
            expires_at_path: "$.subscription.expires_at".into(),
            limits: vec![
                crate::models::endpoint::BalanceLimitExtraction {
                    label: "今日额度".into(),
                    balance_path: "$.subscription.daily_limit_usd - $.subscription.daily_usage_usd"
                        .into(),
                    used_path: "$.subscription.daily_usage_usd".into(),
                    expires_at_path: "$.subscription.expires_at".into(),
                },
                crate::models::endpoint::BalanceLimitExtraction {
                    label: "每周额度".into(),
                    balance_path:
                        "$.subscription.weekly_limit_usd - $.subscription.weekly_usage_usd".into(),
                    used_path: "$.subscription.weekly_usage_usd".into(),
                    expires_at_path: "$.subscription.expires_at".into(),
                },
                crate::models::endpoint::BalanceLimitExtraction {
                    label: "每月额度".into(),
                    balance_path:
                        "$.subscription.monthly_limit_usd - $.subscription.monthly_usage_usd".into(),
                    used_path: "$.subscription.monthly_usage_usd".into(),
                    expires_at_path: "$.subscription.expires_at".into(),
                },
            ],
        },
    }
}

fn newapi_like_balance_config(path: &str, balance_path: &str) -> BalanceQueryConfig {
    BalanceQueryConfig {
        enabled: true,
        template_id: "relay-auto".into(),
        method: "GET".into(),
        path: path.into(),
        headers: vec![crate::models::endpoint::BalanceHeader {
            name: "Authorization".into(),
            value: "Bearer {{apiKey}}".into(),
        }],
        body: String::new(),
        extraction: crate::models::endpoint::BalanceExtraction {
            balance_path: balance_path.into(),
            currency_path: "$.data.currency".into(),
            used_path: "$.data.used_quota".into(),
            expires_at_path: "$.data.expired_time".into(),
            limits: Vec::new(),
        },
    }
}

fn balance_query_timeout() -> Duration {
    Duration::from_secs(8)
}

pub async fn query_endpoint_balance(
    state: State<'_, AppState>,
    id: i64,
) -> AppResult<BalanceQueryResult> {
    let ep = {
        let conn = state.db_pool.get()?;
        endpoint_repo::get_by_id(&conn, id)?
            .ok_or_else(|| AppError::NotFound(format!("端点 #{id} 不存在")))?
    };
    let cfg = ep.balance_query.clone();
    if !cfg.enabled {
        return Err(AppError::InvalidArgument("该端点未启用余额查询模板".into()));
    }
    let first = query_endpoint_balance_with_config(state.clone(), ep.clone(), cfg).await?;
    if !should_try_balance_presets(&first) {
        return Ok(first);
    }

    for preset in balance_query_presets() {
        let template_id = preset.template_id.clone();
        let result =
            query_endpoint_balance_with_config(state.clone(), ep.clone(), preset.clone()).await?;
        if result.success {
            let conn = state.db_pool.get()?;
            let req = UpdateEndpointRequest {
                balance_query: Some(preset),
                ..Default::default()
            };
            let _ = endpoint_repo::update(&conn, ep.id, &req)?;
            return Ok(mark_balance_template_matched(result, &template_id));
        }
    }

    Ok(first)
}

pub async fn test_endpoint_balance_query(
    state: State<'_, AppState>,
    id: i64,
    balance_query: BalanceQueryConfig,
) -> AppResult<BalanceQueryResult> {
    let ep = {
        let conn = state.db_pool.get()?;
        endpoint_repo::get_by_id(&conn, id)?
            .ok_or_else(|| AppError::NotFound(format!("端点 #{id} 不存在")))?
    };
    query_endpoint_balance_with_config(state, ep, balance_query).await
}

async fn query_endpoint_balance_with_config(
    state: State<'_, AppState>,
    ep: Endpoint,
    cfg: BalanceQueryConfig,
) -> AppResult<BalanceQueryResult> {
    let (proxy_enabled, proxy_url, openai_ua, claude_cli_ua) = {
        let conn = state.db_pool.get()?;
        let cfg = config_repo::get_config(&conn)?;
        (
            cfg.proxy_enabled,
            cfg.proxy_url,
            cfg.openai_ua,
            cfg.claude_cli_ua,
        )
    };
    let want = should_use_proxy(ep.use_proxy, proxy_enabled, &proxy_url);
    let client = build_client(want, &proxy_url, balance_query_timeout())?;
    run_balance_query(&client, &ep, &cfg, Some(&openai_ua), Some(&claude_cli_ua)).await
}

/// 探测端点连通性：发送最小请求，200 即可用；持久化 test_status。
pub async fn probe_endpoint_balance_templates(
    state: State<'_, AppState>,
    id: i64,
    custom_path: Option<String>,
) -> AppResult<BalanceProbeResult> {
    let ep = {
        let conn = state.db_pool.get()?;
        endpoint_repo::get_by_id(&conn, id)?
            .ok_or_else(|| AppError::NotFound(format!("端点 #{id} 不存在")))?
    };
    let (proxy_enabled, proxy_url, openai_ua, claude_cli_ua) = {
        let conn = state.db_pool.get()?;
        let cfg = config_repo::get_config(&conn)?;
        (
            cfg.proxy_enabled,
            cfg.proxy_url,
            cfg.openai_ua,
            cfg.claude_cli_ua,
        )
    };
    let want = should_use_proxy(ep.use_proxy, proxy_enabled, &proxy_url);
    let client = build_client(want, &proxy_url, balance_query_timeout())?;
    let templates = if let Some(path) = custom_path.filter(|v| !v.trim().is_empty()) {
        vec![custom_probe_config(path)]
    } else {
        balance_query_presets()
    };
    let mut results = Vec::with_capacity(templates.len());

    for template in templates {
        let path = template.path.clone();
        let template_id = template.template_id.clone();
        let result = match run_balance_query(
            &client,
            &ep,
            &template,
            Some(&openai_ua),
            Some(&claude_cli_ua),
        )
        .await
        {
            Ok(query) => {
                let sample = if query.status < 400 {
                    Some(sanitize_balance_sample(&query.raw, &ep))
                } else {
                    None
                };
                BalanceProbeTemplateResult {
                    template_id,
                    path,
                    success: query.success,
                    url_reachable: query.status < 400,
                    status_code: Some(query.status),
                    latency_ms: query.latency_ms,
                    message: query.message,
                    sample,
                    config: if query.success {
                        Some(template.clone())
                    } else {
                        None
                    },
                    balance: query.balance,
                }
            }
            Err(error) => BalanceProbeTemplateResult {
                template_id,
                path,
                success: false,
                url_reachable: false,
                status_code: None,
                latency_ms: 0,
                message: error.to_string(),
                sample: None,
                config: None,
                balance: None,
            },
        };
        results.push(result);
    }

    Ok(classify_balance_probe_results(results))
}

pub async fn generate_balance_template_with_ai(
    state: State<'_, AppState>,
    id: i64,
    ai_model: String,
    samples: Vec<BalanceTemplateAiSample>,
) -> AppResult<BalanceQueryConfig> {
    let samples: Vec<BalanceTemplateAiSample> = samples
        .into_iter()
        .filter(|sample| !sample.sample.as_deref().unwrap_or("").trim().is_empty())
        .collect();
    if samples.is_empty() {
        return Err(AppError::InvalidArgument(
            "没有可用的余额接口返回样本，不能调用 AI 生成模板".into(),
        ));
    }
    let target = {
        let conn = state.db_pool.get()?;
        endpoint_repo::get_by_id(&conn, id)?
            .ok_or_else(|| AppError::NotFound(format!("端点 #{id} 不存在")))?
    };
    let ai_model = ai_model.trim().to_string();
    if ai_model.is_empty() {
        return Err(AppError::InvalidArgument("请选择此站点下的 AI 模型".into()));
    }
    let available_models: Vec<String> = std::iter::once(target.model.clone())
        .chain(target.models.clone())
        .map(|model| model.trim().to_string())
        .filter(|model| !model.is_empty())
        .collect();
    if available_models.is_empty() {
        return Err(AppError::InvalidArgument(
            "此站点下没有模型，不能使用智能 AI 识别".into(),
        ));
    }
    if !available_models.iter().any(|model| model == &ai_model) {
        return Err(AppError::InvalidArgument(
            "选择的 AI 模型不属于此站点".into(),
        ));
    }
    let (proxy_enabled, proxy_url, openai_ua, claude_cli_ua) = {
        let conn = state.db_pool.get()?;
        let cfg = config_repo::get_config(&conn)?;
        (
            cfg.proxy_enabled,
            cfg.proxy_url,
            cfg.openai_ua,
            cfg.claude_cli_ua,
        )
    };
    let want = should_use_proxy(target.use_proxy, proxy_enabled, &proxy_url);
    let client = build_client(want, &proxy_url, Duration::from_secs(60))?;
    let format = UpstreamFormat::from_transformer_name(&target.transformer);
    let prompt = balance_template_prompt(&target, &samples);
    let url = ai_chat_url(&target.api_url, format);
    let body = match format {
        UpstreamFormat::OpenAiChat => json!({
            "model": ai_model,
            "temperature": 0,
            "messages": [
                { "role": "system", "content": "Return strict JSON only." },
                { "role": "user", "content": prompt }
            ]
        }),
        UpstreamFormat::OpenAiResponses => json!({
            "model": ai_model,
            "temperature": 0,
            "input": prompt
        }),
        UpstreamFormat::Claude => json!({
            "model": ai_model,
            "max_tokens": 1200,
            "temperature": 0,
            "messages": [
                { "role": "user", "content": prompt }
            ]
        }),
    };
    let resp = ProbeAuth::primary_for(&target.transformer)
        .apply_with_ua(
            client.post(url),
            &target.api_key,
            Some(&openai_ua),
            Some(&claude_cli_ua),
        )
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Proxy(format!("调用 AI 生成余额模板失败: {e}")))?;
    let status = resp.status().as_u16();
    let raw = resp
        .text()
        .await
        .map_err(|e| AppError::Proxy(format!("读取 AI 响应失败: {e}")))?;
    if status >= 400 {
        return Err(AppError::Proxy(format!("AI 端点返回 HTTP {status}: {raw}")));
    }
    let value: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| AppError::Proxy(format!("AI 响应不是 JSON: {e}")))?;
    let text = extract_ai_text(format, &value)
        .ok_or_else(|| AppError::Proxy("AI 响应中未找到文本内容".into()))?;
    let cfg = parse_ai_balance_config(&text)?;
    if balance_config_extracts_from_samples(&cfg, &samples) {
        return Ok(cfg);
    }
    if let Some(inferred) = infer_balance_config_from_samples(&samples) {
        return Ok(inferred);
    }
    Err(AppError::InvalidArgument(
        "AI 生成的余额模板无法从返回样本中提取余额，请检查 JSON Path 后再测试".into(),
    ))
}

fn endpoint_test_kind(format: UpstreamFormat) -> &'static str {
    match format {
        UpstreamFormat::OpenAiChat => "openai /v1/chat/completions",
        UpstreamFormat::OpenAiResponses => "codex /v1/responses",
        UpstreamFormat::Claude => "claude /v1/messages",
    }
}

fn should_retry_endpoint_test_status(code: u16) -> bool {
    matches!(code, 403 | 408 | 409 | 425 | 429 | 500..=599)
}

#[cfg(test)]
fn endpoint_test_http_error_message(code: u16, url: &str, model: &str, body: &str) -> String {
    diagnose_upstream_error(code, body).format_for_endpoint_test(code, url, model)
}

#[derive(Debug, Clone)]
struct EndpointProbeLog {
    url: String,
    body: Value,
    request_headers: Vec<RequestTraceHeader>,
    status_code: Option<u16>,
    response_headers: Vec<RequestTraceHeader>,
    response_body: Option<String>,
    error_message: Option<String>,
}

fn endpoint_probe_request_headers(
    auth: ProbeAuth,
    api_key: &str,
    openai_ua: Option<&str>,
    claude_ua: Option<&str>,
) -> Vec<RequestTraceHeader> {
    use crate::utils::ua;

    let mut headers = vec![("content-type".to_string(), "application/json".to_string())];
    match auth {
        ProbeAuth::Bearer => {
            headers.push((
                "user-agent".to_string(),
                openai_ua
                    .filter(|v| !v.trim().is_empty())
                    .and_then(ua::usable_openai_codex_ua)
                    .map(str::to_string)
                    .unwrap_or_else(ua::codex_probe_ua),
            ));
            headers.push(("originator".to_string(), ua::CODEX_ORIGINATOR.to_string()));
            headers.push(("authorization".to_string(), format!("Bearer {api_key}")));
        }
        ProbeAuth::Claude => {
            headers.push((
                "user-agent".to_string(),
                claude_ua
                    .filter(|v| !v.trim().is_empty())
                    .unwrap_or(ua::CLAUDE_PROBE_UA)
                    .to_string(),
            ));
            headers.push(("x-api-key".to_string(), api_key.to_string()));
            headers.push(("anthropic-version".to_string(), "2023-06-01".to_string()));
        }
    }
    trace_capture::capture_header_pairs(&headers)
}

fn raw_upstream_endpoint_error(
    _kind: &str,
    status: u16,
    url: &str,
    model: &str,
    body: &str,
) -> String {
    let body = body.trim();
    let path = endpoint_test_url_path(url);
    if status == 200 && body.is_empty() {
        return format!("{path}，模型: {model} 测试失败: 请求成功但未返回实际流响应结果");
    }
    let reason = extract_upstream_error_message(body)
        .unwrap_or_else(|| truncate_endpoint_message_body(body, 160));
    if status == 200 {
        if reason.is_empty() {
            return format!("{path}，模型: {model} 测试失败: 请求成功但未返回实际流响应结果");
        }
        return format!("{path}，模型: {model} 测试失败: {reason}");
    }
    if body.is_empty() {
        return format!("{path}，模型: {model} 测试失败: 上游返回 {status}: 响应体为空");
    }
    format!("{path}，模型: {model} 测试失败: 上游返回 {status}: {reason}")
}

fn endpoint_test_url_path(url: &str) -> String {
    let path = url
        .split_once("://")
        .and_then(|(_, rest)| rest.find('/').map(|idx| &rest[idx..]))
        .unwrap_or(url)
        .trim_start_matches('/');
    if let Some(idx) = path.find("v1/") {
        path[idx..].to_string()
    } else {
        path.to_string()
    }
}

fn stream_probe_has_actual_output(text: &str, marker: &str) -> bool {
    if !text.contains(marker) {
        return false;
    }
    for line in text.lines() {
        let data = line.trim().strip_prefix("data:").map(str::trim);
        let Some(data) = data else {
            continue;
        };
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        if data.contains("response.output_text.delta")
            || data.contains("\"delta\"")
            || data.contains("content_block_delta")
            || data.contains("message_delta")
            || data.contains("chat.completion.chunk")
        {
            return true;
        }
        if data.contains("\"choices\"") && data.contains("\"content\"") {
            return true;
        }
        if data.contains("\"type\":\"message\"") && data.contains("\"content\"") {
            return true;
        }
    }
    false
}

fn truncate_endpoint_message_body(body: &str, max_chars: usize) -> String {
    let trimmed = body.trim();
    let mut out: String = trimmed.chars().take(max_chars).collect();
    if trimmed.chars().count() > max_chars {
        out.push('…');
    }
    out
}

fn strip_trace_suffix(message: &str) -> String {
    let mut text = message.trim().to_string();
    for marker in ["（traceid:", "(traceid:", "（trace_id:", "(trace_id:"] {
        if let Some(idx) = text.to_lowercase().find(marker) {
            text.truncate(idx);
            break;
        }
    }
    text.trim()
        .trim_end_matches('。')
        .trim_end_matches('.')
        .trim()
        .to_string()
}

fn extract_upstream_error_message(body: &str) -> Option<String> {
    if let Some(message) = extract_upstream_error_message_from_json(body) {
        return Some(message);
    }
    for line in body.lines() {
        let Some(data) = line.trim().strip_prefix("data:").map(str::trim) else {
            continue;
        };
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        if let Some(message) = extract_upstream_error_message_from_json(data) {
            return Some(message);
        }
    }
    None
}

fn extract_upstream_error_message_from_json(body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    let candidates = [
        value.pointer("/error/message"),
        value.pointer("/message"),
        value.pointer("/error"),
    ];
    for candidate in candidates.into_iter().flatten() {
        if let Some(message) = candidate.as_str() {
            let message = strip_trace_suffix(message);
            if !message.is_empty() {
                return Some(message);
            }
        }
    }
    None
}

fn upstream_read_error_endpoint_message(
    kind: &str,
    url: &str,
    model: &str,
    _error: &str,
) -> String {
    raw_upstream_endpoint_error(kind, 200, url, model, "")
}

fn endpoint_test_should_run_long_probe(format: UpstreamFormat, deep: bool) -> bool {
    deep && matches!(format, UpstreamFormat::OpenAiResponses)
}

fn endpoint_connect_error_message(url: &str, model: &str) -> String {
    format!(
        "{}，模型: {model} 测试失败: 无法连接到上游",
        endpoint_test_url_path(url)
    )
}

fn endpoint_timeout_error_message(url: &str, model: &str) -> String {
    let suffix = if endpoint_test_url_path(url).contains("v1/responses") {
        "请求已发出，但上游未在测试超时时间内返回实际流响应结果"
    } else {
        "请求已发出，但上游未在测试超时时间内返回实际响应结果"
    };
    format!(
        "{}，模型: {model} 测试失败: {suffix}",
        endpoint_test_url_path(url),
    )
}

fn select_endpoint_probe_model(ep: &Endpoint, format: UpstreamFormat) -> String {
    let locked = ep.model.trim();
    if !locked.is_empty() {
        return locked.to_string();
    }
    ep.models
        .iter()
        .find(|m| !m.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| format.default_model().to_string())
}

async fn send_endpoint_probe(
    client: &reqwest::Client,
    transformer: &str,
    api_key: &str,
    url: &str,
    body: &Value,
    marker: Option<&str>,
    kind: &str,
    model: &str,
    openai_ua: Option<&str>,
    claude_ua: Option<&str>,
) -> (Result<(), String>, EndpointProbeLog) {
    let auth = ProbeAuth::primary_for(transformer);
    let request_headers = endpoint_probe_request_headers(auth, api_key, openai_ua, claude_ua);
    let request = auth
        .apply_with_ua(client.post(url), api_key, openai_ua, claude_ua)
        .json(body);
    let resp = match request.send().await {
        Ok(resp) => resp,
        Err(e) => {
            let message = if e.is_timeout() {
                endpoint_timeout_error_message(url, model)
            } else {
                endpoint_connect_error_message(url, model)
            };
            let detail = format!("测试失败：无法连接到上游 {url}: {e}");
            return (
                Err(message.clone()),
                EndpointProbeLog {
                    url: url.to_string(),
                    body: body.clone(),
                    request_headers,
                    status_code: None,
                    response_headers: Vec::new(),
                    response_body: None,
                    error_message: Some(detail),
                },
            );
        }
    };
    let code = resp.status().as_u16();
    let response_headers = trace_capture::capture_headers(resp.headers());
    let text = resp.text().await.unwrap_or_default();
    let log = EndpointProbeLog {
        url: url.to_string(),
        body: body.clone(),
        request_headers,
        status_code: Some(code),
        response_headers,
        response_body: Some(text.clone()),
        error_message: None,
    };
    if code == 200 {
        if let Some(marker) = marker {
            if stream_probe_has_actual_output(&text, marker) {
                return (Ok(()), log);
            }
            return (
                Err(raw_upstream_endpoint_error(kind, code, url, model, &text)),
                log,
            );
        }
        if text.trim().is_empty() {
            return (
                Err(raw_upstream_endpoint_error(kind, code, url, model, &text)),
                log,
            );
        }
        return (Ok(()), log);
    }
    (
        Err(raw_upstream_endpoint_error(kind, code, url, model, &text)),
        log,
    )
}

fn endpoint_test_probe(
    base: &str,
    format: UpstreamFormat,
    model: &str,
) -> (String, Value, Option<&'static str>) {
    match format {
        UpstreamFormat::OpenAiChat => (
            format!("{base}/v1/chat/completions"),
            json!({
                "model": model, "max_tokens": 16, "stream": false,
                "messages": [{ "role": "user", "content": "ping" }]
            }),
            None,
        ),
        UpstreamFormat::OpenAiResponses => (
            format!("{base}/v1/responses"),
            json!({
                "model": model,
                "instructions": "You are GPT.",
                "max_output_tokens": 64,
                "parallel_tool_calls": true,
                "reasoning": { "effort": "medium" },
                "store": false,
                "stream": true,
                "tools": [
                    {
                        "type": "function",
                        "name": "ccmesh_probe",
                        "description": "Connectivity probe tool. Do not call unless needed.",
                        "parameters": {
                            "type": "object",
                            "properties": {},
                            "additionalProperties": false
                        },
                        "strict": true
                    }
                ],
                "tool_choice": "auto",
                "input": [
                    {
                        "role": "user",
                        "content": [
                            {
                                "type": "input_text",
                                "text": "请简短回答 OK"
                            }
                        ]
                    }
                ]
            }),
            Some("response.completed"),
        ),
        UpstreamFormat::Claude => (
            format!("{base}/v1/messages"),
            json!({
                "model": model, "max_tokens": 16,
                "messages": [{ "role": "user", "content": "ping" }]
            }),
            None,
        ),
    }
}

fn endpoint_test_long_responses_probe(
    base: &str,
    model: &str,
) -> (String, Value, Option<&'static str>) {
    let long_text = "长上下文测试 ".repeat(12_000);
    (
        format!("{base}/v1/responses"),
        json!({
            "model": model,
            "instructions": "You are GPT. Read the input and return exactly OK.",
            "max_output_tokens": 64,
            "parallel_tool_calls": true,
            "reasoning": { "effort": "medium" },
            "store": false,
            "stream": true,
            "tools": [
                {
                    "type": "function",
                    "name": "ccmesh_probe",
                    "description": "Connectivity probe tool. Do not call unless needed.",
                    "parameters": {
                        "type": "object",
                        "properties": {},
                        "additionalProperties": false
                    },
                    "strict": true
                }
            ],
            "tool_choice": "auto",
            "input": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "input_text",
                            "text": format!("请阅读下面内容后只回复 OK。\n{long_text}")
                        }
                    ]
                }
            ]
        }),
        Some("response.completed"),
    )
}

fn pretty_json_body(value: &Value) -> Option<String> {
    serde_json::to_string_pretty(value).ok()
}

fn record_endpoint_test_log(
    state: &AppState,
    endpoint_name: &str,
    model: &str,
    probe: Option<&EndpointProbeLog>,
    success: bool,
    latency_ms: u64,
    message: &str,
) {
    let status_code = probe.and_then(|item| item.status_code).map(i64::from);
    let upstream_url = probe.map(|item| item.url.clone()).unwrap_or_default();
    let upstream_path = upstream_url
        .split_once("://")
        .and_then(|(_, rest)| rest.find('/').map(|idx| rest[idx..].to_string()))
        .unwrap_or_default();
    let request_body = probe.and_then(|item| pretty_json_body(&item.body));
    let upstream_body = probe.and_then(|item| {
        item.response_body
            .clone()
            .or_else(|| item.error_message.clone())
    });
    let error_body = if success {
        None
    } else {
        Some(message.to_string())
    };

    state.stats.record(RequestRecord {
        endpoint_name: endpoint_name.to_string(),
        model: Some(model.to_string()),
        inbound_format: "endpoint-test".to_string(),
        transformer: None,
        upstream_url,
        inbound_path: "__admin/test_endpoint".to_string(),
        upstream_path,
        status_code,
        is_error: !success,
        usage: TokenUsage::default(),
        duration_ms: Some(latency_ms as i64),
        first_byte_ms: None,
        actual_model: None,
        error_body,
        trace: Some(RequestTrace {
            received_request: RequestTraceStage {
                method: Some("POST".to_string()),
                url: Some("__admin/test_endpoint".to_string()),
                status_code: None,
                headers: trace_capture::json_headers(),
                body: Some(format!("endpoint={endpoint_name}, model={model}")),
            },
            forward_request: RequestTraceStage {
                method: Some("POST".to_string()),
                url: probe.map(|item| item.url.clone()),
                status_code: None,
                headers: probe
                    .map(|item| item.request_headers.clone())
                    .unwrap_or_else(trace_capture::json_headers),
                body: request_body,
            },
            received_forwarded_request: RequestTraceStage {
                method: None,
                url: probe.map(|item| item.url.clone()),
                status_code,
                headers: probe
                    .map(|item| item.response_headers.clone())
                    .unwrap_or_default(),
                body: upstream_body,
            },
            response_request: RequestTraceStage {
                method: None,
                url: Some("__admin/test_endpoint".to_string()),
                status_code: Some(if success { 200 } else { 502 }),
                headers: trace_capture::json_headers(),
                body: Some(message.to_string()),
            },
        }),
    });
}

pub async fn test_endpoint(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    model: Option<String>,
    mode: Option<String>,
) -> AppResult<TestResult> {
    let ep = {
        let conn = state.db_pool.get()?;
        endpoint_repo::get_by_id(&conn, id)?
            .ok_or_else(|| AppError::NotFound(format!("端点 #{id} 不存在")))?
    };

    // 测试 client 遵循代理决策：端点 use_proxy 或全局 proxyEnabled（且地址非空）则经代理，否则直连。
    let (proxy_enabled, proxy_url, openai_ua, claude_cli_ua) = {
        let conn = state.db_pool.get()?;
        let cfg = config_repo::get_config(&conn)?;
        (
            cfg.proxy_enabled,
            cfg.proxy_url,
            cfg.openai_ua,
            cfg.claude_cli_ua,
        )
    };
    let want = should_use_proxy(ep.use_proxy, proxy_enabled, &proxy_url);
    let base = ep.api_url.trim_end_matches('/');
    let format = UpstreamFormat::from_transformer_name(&ep.transformer);
    let deep = mode
        .as_deref()
        .map(|v| v.eq_ignore_ascii_case("deep"))
        .unwrap_or(false);
    let client_timeout = if deep {
        Duration::from_secs(30)
    } else if matches!(format, UpstreamFormat::OpenAiResponses) {
        Duration::from_secs(12)
    } else {
        Duration::from_secs(30)
    };
    let client = build_client(want, &proxy_url, client_timeout)?;

    // 模型优先级：本次测试临时选择 > 端点锁定模型 > 端点模型列表首选 > 格式默认模型。
    let requested_model = model
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| select_endpoint_probe_model(&ep, format));
    let outbound_model =
        crate::modules::proxy::resolver::resolve_outbound(&ep, Some(&requested_model))
            .unwrap_or(requested_model);
    let model = outbound_model.as_str();
    let max_attempts = if deep {
        ENDPOINT_TEST_MAX_ATTEMPTS
    } else {
        ENDPOINT_TEST_QUICK_ATTEMPTS
    };

    let (url, body, stream_marker) = endpoint_test_probe(base, format, model);
    let start = Instant::now();
    let mut attempt = 0usize;
    let mut last_message = String::new();
    let mut success = false;
    let mut status = "unavailable";
    let require_all_stream_attempts = stream_marker.is_some();
    let mut last_probe_log: Option<EndpointProbeLog> = None;
    while attempt < max_attempts {
        attempt += 1;
        let auth = ProbeAuth::primary_for(&ep.transformer);
        let request_headers = endpoint_probe_request_headers(
            auth,
            &ep.api_key,
            Some(&openai_ua),
            Some(&claude_cli_ua),
        );
        let request = auth
            .apply_with_ua(
                client.post(&url),
                &ep.api_key,
                Some(&openai_ua),
                Some(&claude_cli_ua),
            )
            .json(&body);
        match request.send().await {
            Ok(resp) => {
                let code = resp.status().as_u16();
                let response_headers = trace_capture::capture_headers(resp.headers());
                if code == 200 && stream_marker.is_some() {
                    let marker = stream_marker.unwrap();
                    match resp.text().await {
                        Ok(text) if stream_probe_has_actual_output(&text, marker) => {
                            last_probe_log = Some(EndpointProbeLog {
                                url: url.clone(),
                                body: body.clone(),
                                request_headers: request_headers.clone(),
                                status_code: Some(code),
                                response_headers: response_headers.clone(),
                                response_body: Some(text),
                                error_message: None,
                            });
                            last_message = format!(
                                "{}，模型: {model} 短流式探针通过",
                                endpoint_test_url_path(&url),
                            );
                            if !require_all_stream_attempts || attempt >= max_attempts {
                                success = true;
                                status = "available";
                                break;
                            }
                            if attempt < max_attempts {
                                tokio::time::sleep(Duration::from_millis(350)).await;
                            }
                            continue;
                        }
                        Ok(text) => {
                            last_probe_log = Some(EndpointProbeLog {
                                url: url.clone(),
                                body: body.clone(),
                                request_headers: request_headers.clone(),
                                status_code: Some(code),
                                response_headers: response_headers.clone(),
                                response_body: Some(text.clone()),
                                error_message: None,
                            });
                            last_message = raw_upstream_endpoint_error(
                                endpoint_test_kind(format),
                                code,
                                &url,
                                model,
                                &text,
                            );
                            break;
                        }
                        Err(e) => {
                            let message = upstream_read_error_endpoint_message(
                                endpoint_test_kind(format),
                                &url,
                                model,
                                &e.to_string(),
                            );
                            last_probe_log = Some(EndpointProbeLog {
                                url: url.clone(),
                                body: body.clone(),
                                request_headers: request_headers.clone(),
                                status_code: Some(code),
                                response_headers: response_headers.clone(),
                                response_body: None,
                                error_message: Some(format!(
                                    "{}: 读取上游响应失败: {e}",
                                    raw_upstream_endpoint_error(
                                        endpoint_test_kind(format),
                                        code,
                                        &url,
                                        model,
                                        "",
                                    )
                                )),
                            });
                            last_message = message;
                            break;
                        }
                    }
                } else if code == 200 {
                    success = true;
                    status = "available";
                    let text = resp.text().await.unwrap_or_default();
                    last_probe_log = Some(EndpointProbeLog {
                        url: url.clone(),
                        body: body.clone(),
                        request_headers: request_headers.clone(),
                        status_code: Some(code),
                        response_headers: response_headers.clone(),
                        response_body: Some(text),
                        error_message: None,
                    });
                    last_message =
                        format!("{}，模型: {model} 测试成功", endpoint_test_url_path(&url),);
                    break;
                }

                let text = resp.text().await.unwrap_or_default();
                last_probe_log = Some(EndpointProbeLog {
                    url: url.clone(),
                    body: body.clone(),
                    request_headers: request_headers.clone(),
                    status_code: Some(code),
                    response_headers: response_headers.clone(),
                    response_body: Some(text.clone()),
                    error_message: None,
                });
                last_message = raw_upstream_endpoint_error(
                    endpoint_test_kind(format),
                    code,
                    &url,
                    model,
                    &text,
                );
                if !should_retry_endpoint_test_status(code) {
                    break;
                }
            }
            Err(e) => {
                last_message = if e.is_timeout() {
                    endpoint_timeout_error_message(&url, model)
                } else {
                    endpoint_connect_error_message(&url, model)
                };
                last_probe_log = Some(EndpointProbeLog {
                    url: url.clone(),
                    body: body.clone(),
                    request_headers,
                    status_code: None,
                    response_headers: Vec::new(),
                    response_body: None,
                    error_message: Some(format!("测试失败：无法连接到上游 {url}: {e}")),
                });
                break;
            }
        }
        if attempt < max_attempts {
            tokio::time::sleep(Duration::from_millis(350)).await;
        }
    }
    if endpoint_test_should_run_long_probe(format, deep) && success {
        let (long_url, long_body, long_marker) = endpoint_test_long_responses_probe(base, model);
        let (long_result, long_log) = send_endpoint_probe(
            &client,
            &ep.transformer,
            &ep.api_key,
            &long_url,
            &long_body,
            long_marker,
            endpoint_test_kind(format),
            model,
            Some(&openai_ua),
            Some(&claude_cli_ua),
        )
        .await;
        last_probe_log = Some(long_log);
        match long_result {
            Ok(()) => {
                last_message = format!(
                    "{}，模型: {model} 测试成功: 流式响应完整",
                    endpoint_test_url_path(&long_url),
                );
            }
            Err(message) => {
                success = false;
                status = "unavailable";
                last_message = message;
            }
        }
    }
    let latency_ms = start.elapsed().as_millis() as u64;
    let message = if success || attempt <= 1 {
        last_message
    } else {
        format!("{last_message}（本轮共尝试 {attempt} 次）")
    };

    record_endpoint_test_log(
        &state,
        &ep.name,
        model,
        last_probe_log.as_ref(),
        success,
        latency_ms,
        &message,
    );

    {
        let conn = state.db_pool.get()?;
        endpoint_repo::set_test_status(&conn, id, status)?;
    }
    emit_endpoints_changed(&app);

    Ok(TestResult {
        success,
        status: status.to_string(),
        latency_ms,
        message,
    })
}

const PROXY_TEST_URL: &str = "https://www.gstatic.com/generate_204";

/// 测试代理连通性：严格经给定代理地址访问连通性 URL（地址无效直接报错，不回落直连以免误判）。
pub async fn test_proxy(url: String) -> AppResult<TestResult> {
    let url = url.trim();
    if url.is_empty() {
        return Ok(TestResult {
            success: false,
            status: "unavailable".to_string(),
            latency_ms: 0,
            message: "未填写代理地址".to_string(),
        });
    }
    let proxy =
        reqwest::Proxy::all(url).map_err(|e| AppError::Proxy(format!("代理地址无效: {e}")))?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .proxy(proxy)
        .build()
        .map_err(|e| AppError::Proxy(format!("构建代理客户端失败: {e}")))?;

    let start = Instant::now();
    let result = client.get(PROXY_TEST_URL).send().await;
    let latency_ms = start.elapsed().as_millis() as u64;

    let (success, status, message) = match result {
        Ok(resp) => {
            let code = resp.status().as_u16();
            if code < 400 {
                (true, "available", format!("代理可用（HTTP {code}）"))
            } else {
                (false, "unavailable", format!("代理返回 HTTP {code}"))
            }
        }
        Err(e) => (false, "unavailable", format!("经代理请求失败: {e}")),
    };

    Ok(TestResult {
        success,
        status: status.to_string(),
        latency_ms,
        message,
    })
}

#[cfg(test)]
mod balance_probe_tests {
    use super::*;

    fn probe_result(
        template_id: &str,
        success: bool,
        reachable: bool,
    ) -> BalanceProbeTemplateResult {
        BalanceProbeTemplateResult {
            template_id: template_id.to_string(),
            path: "/api/user/self".to_string(),
            success,
            url_reachable: reachable,
            status_code: if reachable { Some(200) } else { None },
            latency_ms: 1,
            message: "test".to_string(),
            sample: if reachable {
                Some("{\"data\":{\"quota\":123},\"api_key\":\"sk-secret\"}".to_string())
            } else {
                None
            },
            config: None,
            balance: if success {
                Some("123".to_string())
            } else {
                None
            },
        }
    }

    fn probe_result_with_sample(template_id: &str, sample: &str) -> BalanceProbeTemplateResult {
        BalanceProbeTemplateResult {
            template_id: template_id.to_string(),
            path: "/api/user/self".to_string(),
            success: false,
            url_reachable: true,
            status_code: Some(200),
            latency_ms: 1,
            message: "test".to_string(),
            sample: Some(sample.to_string()),
            config: None,
            balance: None,
        }
    }

    #[test]
    fn classify_probe_prefers_matched_template() {
        let result = classify_balance_probe_results(vec![
            probe_result("openai-credit-grants", false, true),
            probe_result("newapi-user-self", true, true),
        ]);

        assert_eq!(result.status, "matched");
        assert_eq!(result.matched.unwrap().template_id, "newapi-user-self");
        assert!(result.usable_samples.is_empty());
    }

    #[test]
    fn classify_probe_exposes_samples_when_url_works_but_extraction_fails() {
        let result =
            classify_balance_probe_results(vec![probe_result("newapi-user-self", false, true)]);

        assert_eq!(result.status, "sampleAvailable");
        assert_eq!(result.usable_samples.len(), 1);
        assert!(result.usable_samples[0]
            .sample
            .as_ref()
            .unwrap()
            .contains("***"));
        assert!(!result.usable_samples[0]
            .sample
            .as_ref()
            .unwrap()
            .contains("sk-secret"));
    }

    #[test]
    fn classify_probe_blocks_ai_when_all_urls_fail() {
        let result = classify_balance_probe_results(vec![
            probe_result("openai-credit-grants", false, false),
            probe_result("newapi-user-self", false, false),
        ]);

        assert_eq!(result.status, "allFailed");
        assert!(result.matched.is_none());
        assert!(result.usable_samples.is_empty());
    }

    #[test]
    fn classify_probe_filters_html_and_auth_error_samples_from_ai() {
        let result = classify_balance_probe_results(vec![
            probe_result_with_sample("openai", "<!doctype html><html></html>"),
            probe_result_with_sample(
                "newapi",
                r#"{"success":false,"message":"Unauthorized, invalid access token"}"#,
            ),
        ]);

        assert_eq!(result.status, "allFailed");
        assert!(result.matched.is_none());
        assert!(result.usable_samples.is_empty());
    }

    #[test]
    fn classify_probe_filters_data_null_error_samples_from_ai() {
        let result = classify_balance_probe_results(vec![probe_result_with_sample(
            "newapi",
            r#"{"code":0,"data":null,"msg":"该接口未接入公益站独立网关，旧转发链路已关闭"}"#,
        )]);

        assert_eq!(result.status, "allFailed");
        assert!(result.usable_samples.is_empty());
    }

    #[test]
    fn balance_missing_field_message_prefers_upstream_json_message() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{"message":"Unauthorized, invalid access token","success":false}"#,
        )
        .unwrap();

        assert_eq!(
            balance_missing_field_message(
                &json,
                r#"{"message":"Unauthorized, invalid access token","success":false}"#
            ),
            "余额接口返回: Unauthorized, invalid access token"
        );
    }

    #[test]
    fn balance_missing_field_message_reports_html_response() {
        let raw = "<!doctype html><html><body>login</body></html>";

        assert_eq!(
            balance_missing_field_message(&serde_json::Value::Null, raw),
            "余额接口返回 HTML 页面，可能是余额路径不正确、需要网页登录态或被站点风控拦截"
        );
    }

    #[test]
    fn balance_query_timeout_is_short_enough_for_batch_queries() {
        assert!(
            balance_query_timeout() <= Duration::from_secs(10),
            "余额批量查询不能被单个失败站点长时间阻塞"
        );
    }

    #[test]
    fn zero_limit_expression_is_not_displayable_quota() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
              "subscription": {
                "daily_limit_usd": 60,
                "daily_usage_usd": 40,
                "monthly_limit_usd": 0,
                "monthly_usage_usd": 319
              }
            }"#,
        )
        .unwrap();

        assert!(balance_limit_expression_has_positive_cap(
            &json,
            "$.subscription.daily_limit_usd - $.subscription.daily_usage_usd"
        ));
        assert!(!balance_limit_expression_has_positive_cap(
            &json,
            "$.subscription.monthly_limit_usd - $.subscription.monthly_usage_usd"
        ));
        assert!(balance_limit_expression_has_positive_cap(
            &json,
            "$.rate_limits[0].remaining"
        ));
    }

    #[test]
    fn json_path_supports_array_index_and_subtraction() {
        let json: serde_json::Value = serde_json::from_str(
            r#"{
              "remaining": 5.5,
              "subscription": {"weekly_limit_usd": 300, "weekly_usage_usd": 204.25},
              "rate_limits": [{"remaining": 3.8, "used": 1.2}]
            }"#,
        )
        .unwrap();

        assert_eq!(
            json_path_value(&json, "$.rate_limits[0].remaining").as_deref(),
            Some("3.8")
        );
        assert_eq!(
            json_path_value(
                &json,
                "$.subscription.weekly_limit_usd - $.subscription.weekly_usage_usd"
            )
            .as_deref(),
            Some("95.75")
        );
        assert_eq!(
            json_path_value(&json, "$.subscription.weekly_usage_usd / 2").as_deref(),
            Some("102.125")
        );
    }

    #[test]
    fn saved_balance_query_without_extracted_fields_should_try_presets() {
        let result = BalanceQueryResult {
            success: false,
            status: 200,
            latency_ms: 12,
            balance: None,
            currency: None,
            used: None,
            expires_at: None,
            limits: Vec::new(),
            message: "余额响应中未找到余额字段".into(),
            raw: r#"{"remaining":5.5,"unit":"USD"}"#.into(),
        };

        assert!(should_try_balance_presets(&result));
    }

    #[test]
    fn generated_balance_config_must_extract_from_samples() {
        let samples = vec![BalanceTemplateAiSample {
            template_id: "cafecode".into(),
            path: "/v1/usage".into(),
            status_code: Some(200),
            sample: Some(
                r#"{
                  "remaining": 5.5,
                  "unit": "USD",
                  "subscription": {
                    "daily_limit_usd": 60,
                    "daily_usage_usd": 54.2,
                    "weekly_limit_usd": 300,
                    "weekly_usage_usd": 204.2,
                    "expires_at": "2026-07-23T18:23:37+08:00"
                  },
                  "usage": {"today": {"actual_cost": 54.2}}
                }"#
                .into(),
            ),
        }];
        let invalid = BalanceQueryConfig {
            enabled: true,
            template_id: "ai-generated".into(),
            method: "GET".into(),
            path: "/v1/usage".into(),
            headers: vec![],
            body: String::new(),
            extraction: crate::models::endpoint::BalanceExtraction {
                balance_path: "$.data.quota".into(),
                currency_path: "$.data.currency".into(),
                used_path: "$.data.used_quota".into(),
                expires_at_path: String::new(),
                limits: Vec::new(),
            },
        };

        assert!(!balance_config_extracts_from_samples(&invalid, &samples));
        let inferred = infer_balance_config_from_samples(&samples).expect("usage template");
        assert_eq!(inferred.path, "/v1/usage");
        assert_eq!(inferred.extraction.balance_path, "$.remaining");
        assert!(balance_config_extracts_from_samples(&inferred, &samples));
    }

    #[test]
    fn apimart_balance_sample_infers_multi_window_quota_template() {
        let samples = vec![BalanceTemplateAiSample {
            template_id: "apimart".into(),
            path: "/v1/user/balance".into(),
            status_code: Some(200),
            sample: Some(
                r#"{
                  "is_active": true,
                  "balance_3h": 12.5,
                  "balance_1d": 88.8,
                  "used_3h": 1.5,
                  "used_1d": 11.2,
                  "limit_3h": 14,
                  "limit_1d": 100
                }"#
                .into(),
            ),
        }];

        let inferred = infer_balance_config_from_samples(&samples).expect("apimart template");

        assert_eq!(inferred.path, "/v1/user/balance");
        assert_eq!(inferred.extraction.balance_path, "$.balance_1d");
        assert_eq!(inferred.extraction.used_path, "$.used_1d");
        assert_eq!(inferred.extraction.limits.len(), 2);
        assert!(balance_config_extracts_from_samples(&inferred, &samples));
    }

    #[test]
    fn endpoint_test_403_message_explains_transient_or_permission_causes() {
        let message = endpoint_test_http_error_message(
            403,
            "https://example.com/v1/responses",
            "gpt-5.5",
            r#"{"error":"forbidden"}"#,
        );

        assert!(message.contains("HTTP 403"));
        assert!(message.contains("处理方式"));
        assert!(message.contains("不要直接判定 Key 错"));
        assert!(should_retry_endpoint_test_status(403));
    }

    #[test]
    fn endpoint_test_503_is_retryable_and_reported_as_upstream_unstable() {
        let message = endpoint_test_http_error_message(
            503,
            "https://example.com/v1/responses",
            "gpt-5.5",
            r#"{"error":"busy"}"#,
        );

        assert!(message.contains("HTTP 503"));
        assert!(message.contains("不稳定"));
        assert!(message.contains("处理方式"));
        assert!(should_retry_endpoint_test_status(503));
    }

    #[test]
    fn endpoint_test_probe_model_error_tells_user_to_fix_mapping() {
        let message = endpoint_test_http_error_message(
            400,
            "https://example.com/v1/responses",
            "gpt-5.5",
            r#"{"error":{"code":"model_not_found","message":"model gpt-5.5 does not exist"}}"#,
        );

        assert!(message.contains("出站模型名"));
        assert!(message.contains("模型映射"));
        assert!(message.contains("处理方式"));
    }

    #[test]
    fn endpoint_test_format_error_tells_user_to_fix_endpoint_type() {
        let message = endpoint_test_http_error_message(
            400,
            "https://example.com/v1/responses",
            "gpt-5.5",
            r#"{"detail":"Input must be a list"}"#,
        );

        assert!(message.contains("请求格式"));
        assert!(message.contains("端点类型"));
        assert!(message.contains("处理方式"));
    }

    #[test]
    fn endpoint_test_codex_restriction_reports_client_upgrade() {
        let message = endpoint_test_http_error_message(
            403,
            "https://new.sharedchat.cc/codex/v1/responses",
            "gpt-5.2",
            r#"{"error":{"message":"请使用最新版的codex客户端或codex cli调用","type":"invalid_request_error","code":"codex_access_restricted"}}"#,
        );

        assert!(message.contains("Codex 客户端版本"));
        assert!(message.contains("User-Agent"));
        assert!(message.contains("codex_access_restricted"));
        assert!(!message.contains("权限不足"));
        assert!(!message.contains("临时风控"));
    }

    #[test]
    fn endpoint_test_codex_probe_sends_tool_choice_only_with_tools() {
        let (url, body, marker) = endpoint_test_probe(
            "https://example.com/codex",
            UpstreamFormat::OpenAiResponses,
            "gpt-5.5",
        );

        assert_eq!(url, "https://example.com/codex/v1/responses");
        assert_eq!(marker, Some("response.completed"));
        assert_eq!(
            body.get("tool_choice").and_then(|v| v.as_str()),
            Some("auto")
        );
        assert!(body
            .get("tools")
            .and_then(|v| v.as_array())
            .is_some_and(|tools| !tools.is_empty()));
    }

    #[test]
    fn endpoint_test_openai_probe_uses_non_stream_to_avoid_empty_stream_false_negative() {
        let (url, body, marker) = endpoint_test_probe(
            "https://example.com",
            UpstreamFormat::OpenAiChat,
            "qwen3.7-max",
        );

        assert_eq!(url, "https://example.com/v1/chat/completions");
        assert_eq!(marker, None);
        assert_eq!(body.get("stream").and_then(|v| v.as_bool()), Some(false));
    }

    #[test]
    fn endpoint_test_timeout_message_does_not_claim_connection_failure() {
        let message =
            endpoint_timeout_error_message("https://example.com/v1/responses", "qwen3.7-max");

        assert!(message.contains("未在测试超时时间内返回实际流响应结果"));
        assert!(!message.contains("无法连接"));
    }

    #[test]
    fn endpoint_test_tool_choice_error_tells_user_exact_request_fix() {
        let message = endpoint_test_http_error_message(
            200,
            "https://vsllm.com/v1/responses",
            "qwen3.7-max",
            r#"event:
data: {"code":"InvalidParameter","message":"<400> InternalError.Algo.InvalidParameter: When using `tool_choice`, `tools` must be set."}"#,
        );

        assert!(message.contains("tool_choice"));
        assert!(message.contains("tools"));
        assert!(message.contains("去掉 tool_choice"));
    }

    #[test]
    fn endpoint_test_workspace_error_tells_user_to_bind_business_workspace() {
        let message = endpoint_test_http_error_message(
            200,
            "https://vsllm.com/v1/responses",
            "qwen3.7-max",
            r#"event:
data: {"code":"InvalidParameter","message":"Missing required parameter: 'workspaceid'. Please ensure your API key is bound to a business workspace. You can manage your workspace bindings at: https://bailian.console.aliyun.com/cn-beijing?tab=globalset#/efm/api_key","request_id":"x"}"#,
        );

        assert!(message.contains("workspaceid"));
        assert!(message.contains("业务空间"));
        assert!(message.contains("API Key"));
        assert!(!message.contains("上游返回 HTTP 200。处理方式：查看返回体原文"));
    }

    #[test]
    fn endpoint_test_workspace_stream_error_mentions_unstable_attempts() {
        let diag = diagnose_upstream_error(
            200,
            r#"event:
data: {"code":"InvalidParameter","message":"Missing required parameter: 'workspaceid'. Please ensure your API key is bound to a business workspace.","request_id":"x"}"#,
        );
        let message = format!(
            "测试失败：{} 返回 HTTP 200，但流式响应不完整，未看到结束标记 {}。已成功 {}/{} 次，说明该端点流式表现不稳定。{}。处理方式：{}。测试 URL: {}，模型: {}。依据：{}",
            endpoint_test_kind(UpstreamFormat::OpenAiResponses),
            "response.completed",
            1,
            2,
            diag.reason,
            diag.action,
            "https://vsllm.com/v1/responses",
            "qwen3.7-max",
            diag.evidence
        );

        assert!(message.contains("已成功 1/2 次"));
        assert!(message.contains("流式表现不稳定"));
        assert!(message.contains("workspaceid"));
        assert!(message.contains("业务空间"));
    }

    #[test]
    fn endpoint_test_empty_stream_reports_success_without_actual_stream_result() {
        let message = raw_upstream_endpoint_error(
            endpoint_test_kind(UpstreamFormat::OpenAiResponses),
            200,
            "https://example.com/v1/responses",
            "qwen3.7-max",
            "",
        );

        assert!(message.contains("请求成功但未返回实际流响应结果"));
        assert!(!message.contains("网络"));
        assert!(!message.contains("证书"));
    }

    #[test]
    fn endpoint_test_sse_json_error_reports_upstream_message_directly() {
        let message = raw_upstream_endpoint_error(
            endpoint_test_kind(UpstreamFormat::OpenAiResponses),
            200,
            "https://vsllm.com/v1/responses",
            "qwen3.7-max",
            r#"event:
data: {"code":"InvalidParameter","message":"Missing required parameter: 'workspaceid'. Please ensure your API key is bound to a business workspace.","request_id":"x"}"#,
        );

        assert_eq!(
            message,
            "v1/responses，模型: qwen3.7-max 测试失败: Missing required parameter: 'workspaceid'. Please ensure your API key is bound to a business workspace"
        );
    }

    #[test]
    fn endpoint_test_read_error_reports_empty_actual_stream_result() {
        let message = upstream_read_error_endpoint_message(
            endpoint_test_kind(UpstreamFormat::OpenAiResponses),
            "https://example.com/v1/responses",
            "qwen3.7-max",
            "stream ended before response.completed",
        );

        assert!(message.contains("请求成功但未返回实际流响应结果"));
        assert!(!message.contains("读取上游响应失败"));
        assert!(!message.contains("stream ended before response.completed"));
        assert!(!message.contains("网络"));
        assert!(!message.contains("证书"));
    }

    #[test]
    fn stream_probe_requires_marker_and_actual_output() {
        let empty_completed =
            "event: response.completed\ndata: {\"type\":\"response.completed\"}\n\n";
        assert!(!stream_probe_has_actual_output(
            empty_completed,
            "response.completed"
        ));

        let with_delta = "event: response.output_text.delta\ndata: {\"type\":\"response.output_text.delta\",\"delta\":\"OK\"}\n\nevent: response.completed\ndata: {\"type\":\"response.completed\"}\n\n";
        assert!(stream_probe_has_actual_output(
            with_delta,
            "response.completed"
        ));
    }

    #[test]
    fn endpoint_test_failure_summarizes_upstream_error_for_user() {
        let raw = r#"{"error":{"message":"当前账户暂无生效套餐，请前往钱包页面激活订阅","code":"insufficient_user_quota"}}"#;
        let message = raw_upstream_endpoint_error(
            endpoint_test_kind(UpstreamFormat::OpenAiResponses),
            402,
            "https://example.com/v1/responses",
            "qwen3.7-max",
            raw,
        );

        assert_eq!(
            message,
            "v1/responses，模型: qwen3.7-max 测试失败: 上游返回 402: 当前账户暂无生效套餐，请前往钱包页面激活订阅"
        );
        assert!(!message.contains("处理方式"));
        assert!(!message.contains("余额页"));
        assert!(!message.contains("insufficient_user_quota"));
    }

    #[test]
    fn endpoint_test_http_error_message_strips_trace_for_user() {
        let message = raw_upstream_endpoint_error(
            endpoint_test_kind(UpstreamFormat::OpenAiResponses),
            403,
            "https://rawchat.cn/codex/v1/responses",
            "gpt-5.4",
            r#"{"error":{"message":"您当前的Codex额度已用完，请返回网页端查看明细。（traceid: 0HNML7IEM0SIF:00000001）","type":"permission_error","code":"codex_quota_exhausted"}}"#,
        );

        assert_eq!(
            message,
            "v1/responses，模型: gpt-5.4 测试失败: 上游返回 403: 您当前的Codex额度已用完，请返回网页端查看明细"
        );
        assert!(!message.contains("traceid"));
        assert!(!message.contains("codex_quota_exhausted"));
    }

    #[test]
    fn endpoint_test_responses_quick_mode_skips_long_stream_probe() {
        assert!(!endpoint_test_should_run_long_probe(
            UpstreamFormat::OpenAiResponses,
            false
        ));
        assert!(endpoint_test_should_run_long_probe(
            UpstreamFormat::OpenAiResponses,
            true
        ));
        assert!(!endpoint_test_should_run_long_probe(
            UpstreamFormat::OpenAiChat,
            true
        ));
    }
}

fn json_path_segment_value<'a>(
    value: &'a serde_json::Value,
    segment: &str,
) -> Option<&'a serde_json::Value> {
    let mut current = value;
    let mut rest = segment;
    if let Some(index_start) = rest.find('[') {
        let key = &rest[..index_start];
        if !key.is_empty() {
            current = current.get(key)?;
        }
        rest = &rest[index_start..];
    } else {
        return current.get(rest);
    }
    while let Some(after_open) = rest.strip_prefix('[') {
        let end = after_open.find(']')?;
        let index = after_open[..end].parse::<usize>().ok()?;
        current = current.as_array()?.get(index)?;
        rest = &after_open[end + 1..];
    }
    if rest.is_empty() {
        Some(current)
    } else {
        None
    }
}

fn format_decimal(value: f64) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let mut text = format!("{value:.12}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    if text == "-0" {
        "0".into()
    } else {
        text
    }
}
