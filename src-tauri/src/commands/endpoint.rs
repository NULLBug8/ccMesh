use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::{AppHandle, Emitter, State};

use crate::error::{AppError, AppResult};
use crate::models::endpoint::{
    BalanceQueryConfig, CreateEndpointRequest, Endpoint, UpdateEndpointRequest,
};
use crate::modules::models_probe::ProbeAuth;
use crate::modules::proxy::client::{build_client, should_use_proxy};
use crate::modules::proxy::diagnostics::diagnose_upstream_error;
use crate::modules::storage::{config_repo, endpoint_repo};
use crate::modules::transform::transformer::UpstreamFormat;
use crate::state::AppState;

const ENDPOINT_TEST_MAX_ATTEMPTS: usize = 3;

/// 端点配置/测试状态变更事件（payload 为空，前端收到后全量重拉相关查询）。
const ENDPOINTS_CHANGED_EVENT: &str = "endpoints-changed";

fn emit_endpoints_changed(app: &AppHandle) {
    let _ = app.emit(ENDPOINTS_CHANGED_EVENT, ());
    crate::modules::web_admin::bridge::emit(ENDPOINTS_CHANGED_EVENT, &());
}

#[tauri::command]
pub fn list_endpoints(state: State<AppState>) -> AppResult<Vec<Endpoint>> {
    let conn = state.db_pool.get()?;
    endpoint_repo::list_all(&conn)
}

#[tauri::command]
pub fn create_endpoint(state: State<AppState>, req: CreateEndpointRequest) -> AppResult<Endpoint> {
    let conn = state.db_pool.get()?;
    endpoint_repo::create(&conn, &req)
}

#[tauri::command]
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

#[tauri::command]
pub fn delete_endpoint(state: State<AppState>, id: i64) -> AppResult<()> {
    let conn = state.db_pool.get()?;
    endpoint_repo::delete(&conn, id)
}

#[tauri::command]
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
#[tauri::command]
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
                            "$.subscription.daily_limit_usd - $.subscription.daily_usage_usd"
                                .into(),
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
                            "$.subscription.daily_limit_usd - $.subscription.daily_usage_usd"
                                .into(),
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
    true
}

async fn run_balance_query(
    client: &reqwest::Client,
    ep: &Endpoint,
    cfg: &BalanceQueryConfig,
) -> AppResult<BalanceQueryResult> {
    let method =
        reqwest::Method::from_bytes(cfg.method.trim().as_bytes()).unwrap_or(reqwest::Method::GET);
    let url = balance_url(ep, cfg);
    let mut req = client.request(method, &url);
    for header in &cfg.headers {
        let name = header.name.trim();
        if name.is_empty() {
            continue;
        }
        req = req.header(name, render_balance_template(&header.value, ep));
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
    let success = status < 400 && (balance.is_some() || !limits.is_empty());
    let message = if success {
        "余额查询成功".to_string()
    } else if status >= 400 {
        format!("余额接口返回 HTTP {status}")
    } else {
        "余额响应中未找到余额字段".to_string()
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
        if json_path_value(&json, "$.data.quota").is_some() {
            return Some(newapi_like_balance_config(&sample.path, "$.data.quota"));
        }
        if json_path_value(&json, "$.data.balance").is_some() {
            return Some(newapi_like_balance_config(&sample.path, "$.data.balance"));
        }
    }
    None
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
                        "$.subscription.monthly_limit_usd - $.subscription.monthly_usage_usd"
                            .into(),
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

#[tauri::command]
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
    query_endpoint_balance_with_config(state, ep, cfg).await
}

#[tauri::command]
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
    let (proxy_enabled, proxy_url) = {
        let conn = state.db_pool.get()?;
        let cfg = config_repo::get_config(&conn)?;
        (cfg.proxy_enabled, cfg.proxy_url)
    };
    let want = should_use_proxy(ep.use_proxy, proxy_enabled, &proxy_url);
    let client = build_client(want, &proxy_url, Duration::from_secs(30))?;
    run_balance_query(&client, &ep, &cfg).await
}

/// 探测端点连通性：发送最小请求，200 即可用；持久化 test_status。
#[tauri::command]
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
    let (proxy_enabled, proxy_url) = {
        let conn = state.db_pool.get()?;
        let cfg = config_repo::get_config(&conn)?;
        (cfg.proxy_enabled, cfg.proxy_url)
    };
    let want = should_use_proxy(ep.use_proxy, proxy_enabled, &proxy_url);
    let client = build_client(want, &proxy_url, Duration::from_secs(30))?;
    let templates = if let Some(path) = custom_path.filter(|v| !v.trim().is_empty()) {
        vec![custom_probe_config(path)]
    } else {
        balance_query_presets()
    };
    let mut results = Vec::with_capacity(templates.len());

    for template in templates {
        let path = template.path.clone();
        let template_id = template.template_id.clone();
        let result = match run_balance_query(&client, &ep, &template).await {
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

#[tauri::command]
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
    let (proxy_enabled, proxy_url) = {
        let conn = state.db_pool.get()?;
        let cfg = config_repo::get_config(&conn)?;
        (cfg.proxy_enabled, cfg.proxy_url)
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
        .apply(client.post(url), &target.api_key)
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

fn endpoint_test_http_error_message(code: u16, url: &str, model: &str, body: &str) -> String {
    diagnose_upstream_error(code, body).format_for_endpoint_test(code, url, model)
}

#[tauri::command]
pub async fn test_endpoint(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
    model: Option<String>,
) -> AppResult<TestResult> {
    let ep = {
        let conn = state.db_pool.get()?;
        endpoint_repo::get_by_id(&conn, id)?
            .ok_or_else(|| AppError::NotFound(format!("端点 #{id} 不存在")))?
    };

    // 测试 client 遵循代理决策：端点 use_proxy 或全局 proxyEnabled（且地址非空）则经代理，否则直连。
    let (proxy_enabled, proxy_url) = {
        let conn = state.db_pool.get()?;
        let cfg = config_repo::get_config(&conn)?;
        (cfg.proxy_enabled, cfg.proxy_url)
    };
    let want = should_use_proxy(ep.use_proxy, proxy_enabled, &proxy_url);
    let client = build_client(want, &proxy_url, Duration::from_secs(30))?;

    let base = ep.api_url.trim_end_matches('/');
    let format = UpstreamFormat::from_transformer_name(&ep.transformer);
    // 优先用调用方指定的模型（前端选择），否则端点锁定 model，再否则按格式回落默认
    let fallback = format.default_model();
    let requested_model = model.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| {
        if ep.model.is_empty() {
            fallback.to_string()
        } else {
            ep.model.clone()
        }
    });
    let outbound_model =
        crate::modules::proxy::resolver::resolve_outbound(&ep, Some(&requested_model))
            .unwrap_or(requested_model);
    let model = outbound_model.as_str();

    let (url, body, stream_marker) = match format {
        UpstreamFormat::OpenAiChat => (
            format!("{base}/v1/chat/completions"),
            json!({
                "model": model, "max_tokens": 16, "stream": true,
                "messages": [{ "role": "user", "content": "ping" }]
            }),
            Some("[DONE]"),
        ),
        UpstreamFormat::OpenAiResponses => (
            format!("{base}/v1/responses"),
            json!({
                "model": model, "max_output_tokens": 16, "stream": true,
                "input": [
                    {
                        "role": "user",
                        "content": [
                            { "type": "input_text", "text": "ping" }
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
    };
    let start = Instant::now();
    let mut attempt = 0usize;
    let mut last_message = String::new();
    let mut success = false;
    let mut status = "unavailable";
    while attempt < ENDPOINT_TEST_MAX_ATTEMPTS {
        attempt += 1;
        let request = ProbeAuth::primary_for(&ep.transformer)
            .apply(client.post(&url), &ep.api_key)
            .json(&body);
        match request.send().await {
            Ok(resp) => {
                let code = resp.status().as_u16();
                if code == 200 && stream_marker.is_some() {
                    let marker = stream_marker.unwrap();
                    match resp.text().await {
                        Ok(text) if text.contains(marker) => {
                            success = true;
                            status = "available";
                            last_message = format!(
                                "测试成功：{} 流式响应完整，模型 {model} 可用",
                                endpoint_test_kind(format)
                            );
                            break;
                        }
                        Ok(text) => {
                            let diag = diagnose_upstream_error(200, &text);
                            last_message = format!(
                                "测试失败：{} 返回 HTTP 200，但流式响应不完整，未看到结束标记 {marker}。{}。处理方式：{}。如果“测试成功但实际调用失败”，请对比日志里的真实请求体：真实调用可能包含工具、图片、超长上下文、reasoning 参数或模型映射，和轻量测试请求不同。测试 URL: {url}，模型: {model}。依据：{}",
                                endpoint_test_kind(format),
                                diag.reason,
                                diag.action,
                                diag.evidence
                            );
                            break;
                        }
                        Err(e) => {
                            last_message =
                                format!("测试失败：已连接到 {url}，但读取流式响应失败: {e}");
                            break;
                        }
                    }
                } else if code == 200 {
                    success = true;
                    status = "available";
                    last_message = format!("测试成功：{url} 可连接，模型 {model} 可用");
                    break;
                }

                let text = resp.text().await.unwrap_or_default();
                last_message = endpoint_test_http_error_message(code, &url, model, &text);
                if !should_retry_endpoint_test_status(code) {
                    break;
                }
            }
            Err(e) => {
                last_message = format!(
                    "测试失败：无法连接到 {url}。请检查 API 地址、网络/代理和证书配置；错误: {e}"
                );
                break;
            }
        }
        if attempt < ENDPOINT_TEST_MAX_ATTEMPTS {
            tokio::time::sleep(Duration::from_millis(350)).await;
        }
    }
    let latency_ms = start.elapsed().as_millis() as u64;
    let message = if success || attempt <= 1 {
        last_message
    } else {
        format!("{last_message}（已重试 {attempt} 次）")
    };

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

/// 代理连通性检测目标：轻量 204 连通性 URL（经代理 GET，验证代理能出网）。
const PROXY_TEST_URL: &str = "https://www.gstatic.com/generate_204";

/// 测试代理连通性：严格经给定代理地址访问连通性 URL（地址无效直接报错，不回落直连以免误判）。
#[tauri::command]
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
    fn endpoint_test_model_error_tells_user_to_fix_mapping() {
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
