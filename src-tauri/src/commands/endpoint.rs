use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, State};

use crate::error::{AppError, AppResult};
use crate::models::endpoint::{BalanceQueryConfig, CreateEndpointRequest, Endpoint, UpdateEndpointRequest};
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
pub fn reorder_endpoints(app: AppHandle, state: State<AppState>, ordered_ids: Vec<i64>) -> AppResult<()> {
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
    let path = trimmed.strip_prefix("$.").or_else(|| trimmed.strip_prefix('$'))?;
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
    let method = reqwest::Method::from_bytes(cfg.method.trim().as_bytes())
        .unwrap_or(reqwest::Method::GET);
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
