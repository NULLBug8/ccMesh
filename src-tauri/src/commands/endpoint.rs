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
use crate::modules::storage::{config_repo, endpoint_repo};
use crate::modules::transform::transformer::UpstreamFormat;
use crate::state::AppState;

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
    pub message: String,
    pub raw: String,
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
        current = current.get(key)?;
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
            template_id: "openai-credit-grants".into(),
            method: "GET".into(),
            path: "/dashboard/billing/credit_grants".into(),
            headers: vec![],
            body: String::new(),
            extraction: crate::models::endpoint::BalanceExtraction {
                balance_path: "$.total_available".into(),
                currency_path: "$.currency".into(),
                used_path: "$.total_used".into(),
                expires_at_path: "$.expires_at".into(),
            },
        },
        BalanceQueryConfig {
            enabled: true,
            template_id: "newapi-user-self".into(),
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
            },
        },
        BalanceQueryConfig {
            enabled: true,
            template_id: "one-api-self".into(),
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
            },
        },
    ]
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
    let success = status < 400 && balance.is_some();
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
            .filter(|item| item.url_reachable && item.sample.is_some())
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

fn default_ai_model(ep: &Endpoint, format: UpstreamFormat) -> String {
    if !ep.model.trim().is_empty() {
        ep.model.clone()
    } else if let Some(model) = ep.models.iter().find(|m| !m.trim().is_empty()) {
        model.clone()
    } else {
        format.default_model().to_string()
    }
}

fn balance_template_prompt(target: &Endpoint, sample: &BalanceTemplateAiSample) -> String {
    format!(
        r#"You are configuring a relay balance query template.
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
    "expiresAtPath": ""
  }}
}}

Rules:
- Do not include markdown.
- Keep API keys as {{{{apiKey}}}} placeholders.
- Prefer the provided path unless the sample clearly says another path.
- Choose JSON Paths that extract balance, currency, used amount, and expiry when present.

Endpoint name: {endpoint_name}
Endpoint base URL: {api_url}
Probe template: {template_id}
Probe path: {path}
HTTP status: {status}
Sanitized response sample:
{sample}
"#,
        endpoint_name = target.name,
        api_url = target.api_url,
        template_id = sample.template_id,
        path = sample.path,
        status = sample
            .status_code
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".into()),
        sample = sample.sample.clone().unwrap_or_default(),
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
    if cfg.extraction.balance_path.trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "AI 返回的余额模板缺少 balancePath".into(),
        ));
    }
    Ok(cfg)
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

    let (proxy_enabled, proxy_url) = {
        let conn = state.db_pool.get()?;
        let cfg = config_repo::get_config(&conn)?;
        (cfg.proxy_enabled, cfg.proxy_url)
    };
    let want = should_use_proxy(ep.use_proxy, proxy_enabled, &proxy_url);
    let client = build_client(want, &proxy_url, Duration::from_secs(30))?;
    let method =
        reqwest::Method::from_bytes(cfg.method.trim().as_bytes()).unwrap_or(reqwest::Method::GET);
    let url = balance_url(&ep, &cfg);
    let mut req = client.request(method, &url);
    for header in &cfg.headers {
        let name = header.name.trim();
        if name.is_empty() {
            continue;
        }
        req = req.header(name, render_balance_template(&header.value, &ep));
    }
    if !cfg.body.trim().is_empty() {
        req = req.body(render_balance_template(&cfg.body, &ep));
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
    let success = status < 400 && balance.is_some();
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
        message,
        raw,
    })
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
    ai_endpoint_id: i64,
    sample: BalanceTemplateAiSample,
) -> AppResult<BalanceQueryConfig> {
    if sample.sample.as_deref().unwrap_or("").trim().is_empty() {
        return Err(AppError::InvalidArgument(
            "没有可用的余额接口返回样本，不能调用 AI 生成模板".into(),
        ));
    }
    let (target, ai_ep) = {
        let conn = state.db_pool.get()?;
        let target = endpoint_repo::get_by_id(&conn, id)?
            .ok_or_else(|| AppError::NotFound(format!("端点 #{id} 不存在")))?;
        let ai_ep = endpoint_repo::get_by_id(&conn, ai_endpoint_id)?
            .ok_or_else(|| AppError::NotFound(format!("AI 端点 #{ai_endpoint_id} 不存在")))?;
        (target, ai_ep)
    };
    let (proxy_enabled, proxy_url) = {
        let conn = state.db_pool.get()?;
        let cfg = config_repo::get_config(&conn)?;
        (cfg.proxy_enabled, cfg.proxy_url)
    };
    let want = should_use_proxy(ai_ep.use_proxy, proxy_enabled, &proxy_url);
    let client = build_client(want, &proxy_url, Duration::from_secs(60))?;
    let format = UpstreamFormat::from_transformer_name(&ai_ep.transformer);
    let model = default_ai_model(&ai_ep, format);
    let prompt = balance_template_prompt(&target, &sample);
    let url = ai_chat_url(&ai_ep.api_url, format);
    let body = match format {
        UpstreamFormat::OpenAiChat => json!({
            "model": model,
            "temperature": 0,
            "messages": [
                { "role": "system", "content": "Return strict JSON only." },
                { "role": "user", "content": prompt }
            ]
        }),
        UpstreamFormat::OpenAiResponses => json!({
            "model": model,
            "temperature": 0,
            "input": prompt
        }),
        UpstreamFormat::Claude => json!({
            "model": model,
            "max_tokens": 1200,
            "temperature": 0,
            "messages": [
                { "role": "user", "content": prompt }
            ]
        }),
    };
    let resp = ProbeAuth::primary_for(&ai_ep.transformer)
        .apply(client.post(url), &ai_ep.api_key)
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
    parse_ai_balance_config(&text)
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
    let model_str = model.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| {
        if ep.model.is_empty() {
            fallback.to_string()
        } else {
            ep.model.clone()
        }
    });
    let model = model_str.as_str();

    let (url, body) = match format {
        UpstreamFormat::OpenAiChat => (
            format!("{base}/v1/chat/completions"),
            json!({
                "model": model, "max_tokens": 16,
                "messages": [{ "role": "user", "content": "ping" }]
            }),
        ),
        UpstreamFormat::OpenAiResponses => (
            format!("{base}/v1/responses"),
            json!({
                "model": model, "max_output_tokens": 16,
                "input": "ping"
            }),
        ),
        UpstreamFormat::Claude => (
            format!("{base}/v1/messages"),
            json!({
                "model": model, "max_tokens": 16,
                "messages": [{ "role": "user", "content": "ping" }]
            }),
        ),
    };
    let builder = ProbeAuth::primary_for(&ep.transformer)
        .apply(client.post(&url), &ep.api_key)
        .json(&body);

    let start = Instant::now();
    let result = builder.send().await;
    let latency_ms = start.elapsed().as_millis() as u64;

    let (success, status, message) = match result {
        Ok(resp) => {
            let code = resp.status().as_u16();
            if code == 200 {
                (true, "available", "连接成功".to_string())
            } else if code == 401 || code == 403 {
                (false, "unavailable", format!("鉴权失败（HTTP {code}）"))
            } else {
                (false, "unavailable", format!("HTTP {code}"))
            }
        }
        Err(e) => (false, "unavailable", format!("请求失败: {e}")),
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
}
