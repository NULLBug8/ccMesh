use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::body::{Body, Bytes};
use axum::extract::State;
use axum::http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use tokio_util::sync::CancellationToken;

use futures::StreamExt;
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

use crate::models::endpoint::Endpoint;
use crate::models::stats::{RequestTrace, RequestTraceStage};
use crate::modules::proxy::circuit_breaker::{self, BreakerRegistry};
use crate::modules::proxy::diagnostics::diagnose_upstream_error;
use crate::modules::proxy::resolver;
use crate::modules::proxy::rotation::{self, Rotation};
use crate::modules::proxy::trace_capture;
use crate::modules::stats::aggregator::{RequestRecord, StatsAggregator};
use crate::modules::storage::{config_repo, db::DbPool, endpoint_repo};
use crate::modules::transform::claude_openai::openai_response_to_claude;
use crate::modules::transform::reasoning_effort::{
    downgrade_reasoning_effort_in_responses, is_unsupported_reasoning_effort_error,
};
use crate::modules::transform::responses_chat::{
    chat_response_to_responses, responses_request_to_chat, ResponsesStreamConverter,
};
use crate::modules::transform::streaming::StreamConverter;
use crate::modules::transform::thinking_rectifier::{
    rectify_anthropic_request, should_rectify_thinking_signature, RectifierConfig,
};
use crate::modules::transform::transformer::{get_transformer, UpstreamFormat};
use crate::modules::usage;
use crate::utils::ua;

const MAX_ERROR_BODY_BYTES: usize = 4096;

/// 每端点在途请求计数 + 取消令牌（手动切换时中止在途请求）。
#[derive(Default)]
pub struct ActiveRequests {
    inner: Mutex<HashMap<String, ActiveEntry>>,
}

struct ActiveEntry {
    count: usize,
    token: CancellationToken,
}

impl ActiveRequests {
    /// 标记一个请求开始，返回该端点的取消令牌。
    pub fn start(&self, name: &str) -> CancellationToken {
        let mut g = self.inner.lock().unwrap();
        let entry = g.entry(name.to_string()).or_insert_with(|| ActiveEntry {
            count: 0,
            token: CancellationToken::new(),
        });
        entry.count += 1;
        entry.token.clone()
    }

    pub fn finish(&self, name: &str) {
        let mut g = self.inner.lock().unwrap();
        if let Some(e) = g.get_mut(name) {
            e.count = e.count.saturating_sub(1);
        }
    }

    /// 取消该端点所有在途请求并换发新令牌（手动切换用）。
    pub fn cancel(&self, name: &str) {
        let mut g = self.inner.lock().unwrap();
        if let Some(e) = g.get_mut(name) {
            e.token.cancel();
            e.token = CancellationToken::new();
        }
    }
}

/// 代理运行期共享状态，注入 axum 处理器。
pub struct ProxyState {
    pub app: tauri::AppHandle,
    pub db_pool: DbPool,
    pub client: reqwest::Client,
    /// 全局代理 client（配置了 proxy_url 时构建；端点 use_proxy 为真时使用）。
    pub proxy_client: Option<reqwest::Client>,
    /// 伪装 UA：转发到 OpenAI 端点时覆盖 User-Agent（空=透传客户端）。
    pub openai_ua: Mutex<String>,
    /// 伪装 UA：转发到 Claude 端点时覆盖 User-Agent（空=透传客户端）。
    pub claude_cli_ua: Mutex<String>,
    pub rotation: Rotation,
    pub active: ActiveRequests,
    pub stats: Arc<StatsAggregator>,
    pub current_endpoint: Mutex<Option<String>>,
    /// 全局「启用代理」开关（start_proxy 时读配置；端点 use_proxy 未开时按此决定是否走代理）。
    pub proxy_enabled: bool,
    /// 每端点熔断器（请求驱动，运行期内存态）。
    pub breakers: BreakerRegistry,
    /// thinking 签名整流器配置（反应式：上游签名错误时清洗 thinking/signature 后透明重试）。
    pub rectifier_config: RectifierConfig,
    pub reasoning_effort_fallback: bool,
    pub model_mapping_strategy: String,
    pub max_retries: u32,
}

impl ProxyState {
    pub fn current_endpoint_name(&self) -> Option<String> {
        self.current_endpoint.lock().unwrap().clone()
    }
    fn set_current(&self, name: &str) {
        *self.current_endpoint.lock().unwrap() = Some(name.to_string());
    }
}

/// 单次请求的元信息，贯穿响应处理并用于构造统计明细记录。
#[derive(Clone)]
struct RequestMeta {
    endpoint: String,
    model: Option<String>,
    inbound_format: String,
    upstream_url: String,
    /// 真实入站路由路径（客户端实际请求的 `uri.path()`）。
    inbound_path: String,
    /// 真实出站路由路径（实际转发上游的路径，转换时为 `/v1/chat/completions`）。
    upstream_path: String,
    started_ms: i64,
    /// 首字节延迟（毫秒）：响应头到达时置入；流式响应处理器会在首个内容分片到达时覆盖为更精确的首字。
    first_byte_ms: Option<i64>,
    /// 实际(出站)模型：映射/锁定改写后与入站不同才有值，透传为 None。
    actual_model: Option<String>,
    trace: RequestTrace,
}

impl RequestMeta {
    /// 在记录时构造 `RequestRecord`：现在时间减去开始时间得到耗时。
    fn into_record(
        &self,
        status_code: Option<i64>,
        is_error: bool,
        tu: usage::TokenUsage,
    ) -> RequestRecord {
        self.into_record_with_error_body(status_code, is_error, tu, None)
    }

    fn into_record_with_error_body(
        &self,
        status_code: Option<i64>,
        is_error: bool,
        tu: usage::TokenUsage,
        error_body: Option<String>,
    ) -> RequestRecord {
        RequestRecord {
            endpoint_name: self.endpoint.clone(),
            model: self.model.clone(),
            inbound_format: self.inbound_format.clone(),
            upstream_url: self.upstream_url.clone(),
            inbound_path: self.inbound_path.clone(),
            upstream_path: self.upstream_path.clone(),
            status_code,
            is_error,
            usage: tu,
            duration_ms: Some(chrono::Utc::now().timestamp_millis() - self.started_ms),
            first_byte_ms: self.first_byte_ms,
            actual_model: self.actual_model.clone(),
            error_body,
            trace: Some(self.trace.clone()),
        }
    }
}

fn json_error(status: StatusCode, message: &str) -> Response {
    (
        status,
        axum::Json(serde_json::json!({
            "error": { "type": "proxy_error", "message": message }
        })),
    )
        .into_response()
}

fn truncate_error_body(text: &str) -> String {
    if text.len() <= MAX_ERROR_BODY_BYTES {
        return text.to_string();
    }
    let mut end = MAX_ERROR_BODY_BYTES;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}... [已截断，原长度 {} 字节]", &text[..end], text.len())
}

fn error_body_from_bytes(bytes: &Bytes) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    let text = String::from_utf8_lossy(bytes);
    Some(truncate_error_body(&text))
}

fn build_forward_headers(
    st: &ProxyState,
    ep: &Endpoint,
    headers: &HeaderMap,
) -> Vec<(String, String)> {
    let ua_format = UpstreamFormat::from_transformer_name(&ep.transformer);
    let ua_override = match ua_format {
        UpstreamFormat::OpenAiChat | UpstreamFormat::OpenAiResponses => current_openai_ua(st),
        UpstreamFormat::Claude => current_claude_ua(st),
    };
    let ua_override = ua_override.trim().to_string();
    let override_ua = !ua_override.is_empty();
    let add_codex_originator = override_ua
        && ua_override.starts_with(ua::CODEX_ORIGINATOR)
        && !headers.contains_key("originator");

    let mut forward_headers = Vec::new();
    for (key, value) in headers.iter() {
        let lower_name = key.as_str().to_ascii_lowercase();
        if lower_name == "host"
            || lower_name == "content-length"
            || lower_name == "accept-encoding"
            || lower_name == "authorization"
            || lower_name == "x-api-key"
            || lower_name == resolver::ENDPOINT_HEADER
            || lower_name == resolver::ENDPOINT_HEADER_ALT
            || (override_ua && lower_name == "user-agent")
        {
            continue;
        }
        if let Ok(value) = value.to_str() {
            forward_headers.push((key.as_str().to_string(), value.to_string()));
        }
    }

    if override_ua {
        forward_headers.push(("user-agent".to_string(), ua_override.clone()));
    }
    if add_codex_originator {
        forward_headers.push(("originator".to_string(), ua::CODEX_ORIGINATOR.to_string()));
    }

    let key = ep.api_key.as_str();
    if !key.is_empty() {
        match ep.transformer.as_str() {
            "openai" | "openai2" | "openai_chat" => {
                forward_headers.push(("authorization".to_string(), format!("Bearer {key}")));
            }
            _ => {
                forward_headers.push(("x-api-key".to_string(), key.to_string()));
                forward_headers.push(("authorization".to_string(), format!("Bearer {key}")));
            }
        }
    }

    forward_headers
}

fn empty_trace() -> RequestTrace {
    RequestTrace {
        received_request: RequestTraceStage::default(),
        forward_request: RequestTraceStage::default(),
        received_forwarded_request: RequestTraceStage::default(),
        response_request: RequestTraceStage::default(),
    }
}

fn set_upstream_trace(
    meta: &mut RequestMeta,
    status_code: u16,
    headers: Vec<crate::models::stats::RequestTraceHeader>,
    body: Option<String>,
) {
    let url = meta
        .trace
        .forward_request
        .url
        .clone()
        .unwrap_or_else(|| meta.upstream_path.clone());
    meta.trace.received_forwarded_request =
        trace_capture::response_stage(url, Some(status_code as i64), headers, body);
}

fn set_response_trace(
    meta: &mut RequestMeta,
    status_code: u16,
    headers: Vec<crate::models::stats::RequestTraceHeader>,
    body: Option<String>,
) {
    meta.trace.response_request = trace_capture::response_stage(
        meta.inbound_path.clone(),
        Some(status_code as i64),
        headers,
        body,
    );
}

fn empty_candidates_message(model: Option<&str>, breaker_exhausted: bool) -> String {
    if breaker_exhausted {
        return "所有候选端点均熔断或无可用端点".to_string();
    }
    match model.map(str::trim).filter(|m| !m.is_empty()) {
        Some(m) => format!("所有候选端点均不支持模型 '{m}'"),
        None => "所有候选端点均熔断或无可用端点".to_string(),
    }
}

fn should_route_responses_via_chat(
    inbound_responses: bool,
    _client_wants_stream: bool,
    format: UpstreamFormat,
) -> bool {
    if !inbound_responses {
        return false;
    }
    matches!(format, UpstreamFormat::OpenAiChat)
}

fn any_endpoint_declares_model(enabled: &[Endpoint], model: &str) -> bool {
    enabled.iter().any(|ep| {
        resolver::advertised_models(ep)
            .iter()
            .any(|item| item.trim().eq_ignore_ascii_case(model.trim()))
    })
}

fn response_is_event_stream(headers: &HeaderMap) -> bool {
    headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.to_ascii_lowercase().contains("text/event-stream"))
}

fn filter_unavailable_by_test_status(enabled: &[Endpoint]) -> Vec<Endpoint> {
    let filtered: Vec<Endpoint> = enabled
        .iter()
        .filter(|ep| !ep.test_status.eq_ignore_ascii_case("unavailable"))
        .cloned()
        .collect();
    if filtered.is_empty() {
        enabled.to_vec()
    } else {
        filtered
    }
}

fn current_openai_ua(st: &ProxyState) -> String {
    st.openai_ua.lock().unwrap().clone()
}

fn current_claude_ua(st: &ProxyState) -> String {
    st.claude_cli_ua.lock().unwrap().clone()
}

fn is_codex_or_openai_ua(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("codex") || lower.contains("openai")
}

fn is_claude_ua(value: &str) -> bool {
    value.to_ascii_lowercase().contains("claude")
}

fn is_default_or_auto_ua(current: &str, format: UpstreamFormat) -> bool {
    let current = current.trim();
    if current.is_empty() {
        return true;
    }
    match format {
        UpstreamFormat::OpenAiChat | UpstreamFormat::OpenAiResponses => {
            is_codex_or_openai_ua(current)
        }
        UpstreamFormat::Claude => is_claude_ua(current),
    }
}

fn learned_ua_key(
    format: UpstreamFormat,
    incoming_ua: &str,
    current: &str,
) -> Option<&'static str> {
    let incoming_ua = incoming_ua.trim();
    if incoming_ua.is_empty() || incoming_ua == current.trim() {
        return None;
    }
    match format {
        UpstreamFormat::OpenAiChat | UpstreamFormat::OpenAiResponses
            if is_codex_or_openai_ua(incoming_ua) && is_default_or_auto_ua(current, format) =>
        {
            Some("openaiUa")
        }
        UpstreamFormat::Claude
            if is_claude_ua(incoming_ua) && is_default_or_auto_ua(current, format) =>
        {
            Some("claudeCliUa")
        }
        _ => None,
    }
}

fn maybe_learn_client_ua(st: &ProxyState, ep: &Endpoint, headers: &HeaderMap) {
    let Some(incoming_ua) = headers
        .get("user-agent")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return;
    };
    let format = UpstreamFormat::from_transformer_name(&ep.transformer);
    let current = match format {
        UpstreamFormat::OpenAiChat | UpstreamFormat::OpenAiResponses => current_openai_ua(st),
        UpstreamFormat::Claude => current_claude_ua(st),
    };
    let Some(key) = learned_ua_key(format, incoming_ua, &current) else {
        return;
    };
    if let Ok(conn) = st.db_pool.get() {
        if config_repo::set_value(&conn, key, incoming_ua).is_ok() {
            match key {
                "openaiUa" => *st.openai_ua.lock().unwrap() = incoming_ua.to_string(),
                "claudeCliUa" => *st.claude_cli_ua.lock().unwrap() = incoming_ua.to_string(),
                _ => {}
            }
            tracing::info!(key, "已自动学习客户端 User-Agent");
        }
    }
}

fn urldecode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hi = (bytes[i + 1] as char).to_digit(16);
                let lo = (bytes[i + 2] as char).to_digit(16);
                if let (Some(h), Some(l)) = (hi, lo) {
                    out.push((h * 16 + l) as u8);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// 主代理处理器：解析端点 → 轮换/故障转移重试 → 转发上游 → 直通响应。
pub async fn handle_proxy(
    State(st): State<Arc<ProxyState>>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let path = uri.path().to_string();
    let started_ms = chrono::Utc::now().timestamp_millis();
    let received_request_trace =
        trace_capture::request_stage(&method, path.clone(), &headers, &body);

    // 解析请求体（model / stream / 供转换复用）
    let mut body_json: Option<Value> = serde_json::from_slice(&body).ok();
    let model: Option<String> = body_json
        .as_ref()
        .and_then(|v| v.get("model").and_then(|m| m.as_str()).map(String::from));
    let client_wants_stream = body_json
        .as_ref()
        .and_then(|v| v.get("stream").and_then(|s| s.as_bool()))
        .unwrap_or(false);

    // 头部 map（小写键）
    let mut hmap = HashMap::new();
    for (k, v) in headers.iter() {
        if let Ok(val) = v.to_str() {
            hmap.insert(k.as_str().to_ascii_lowercase(), val.to_string());
        }
    }
    // 查询参数 map（小写键）
    let mut qmap = HashMap::new();
    if let Some(q) = uri.query() {
        for pair in q.split('&') {
            if let Some((k, v)) = pair.split_once('=') {
                qmap.insert(k.to_ascii_lowercase(), urldecode(v));
            }
        }
    }

    // 当前启用端点（每请求读取，反映 CRUD 实时变更）
    let enabled = match st.db_pool.get() {
        Ok(conn) => match endpoint_repo::list_enabled(&conn) {
            Ok(list) => list,
            Err(e) => {
                return json_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("加载端点失败: {e}"),
                )
            }
        },
        Err(e) => {
            return json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("数据库连接失败: {e}"),
            )
        }
    };
    if enabled.is_empty() {
        return json_error(StatusCode::SERVICE_UNAVAILABLE, "没有启用的端点");
    }

    // 入站格式识别：/v1/chat/completions = OpenAI Chat 入站；/v1/responses = Responses 入站（codex）。
    let inbound_openai = path.contains("/chat/completions");
    let inbound_responses = path.contains("/responses");
    let enabled: Vec<Endpoint> = if inbound_openai {
        // OpenAI Chat 入站只透传到 OpenAI 端点，故候选过滤为 transformer=openai；为空则 400。
        let filtered: Vec<Endpoint> = enabled
            .into_iter()
            .filter(|e| {
                matches!(
                    UpstreamFormat::from_transformer_name(&e.transformer),
                    UpstreamFormat::OpenAiChat
                )
            })
            .collect();
        if filtered.is_empty() {
            return json_error(
                StatusCode::BAD_REQUEST,
                "OpenAI 入站(/v1/chat/completions)无可用的 OpenAI 端点",
            );
        }
        filtered
    } else if inbound_responses {
        // Responses 入站：codex 端点透传、openai 端点转 Chat。候选 = codex + openai，为空则 400。
        let filtered: Vec<Endpoint> = enabled
            .into_iter()
            .filter(|e| {
                matches!(
                    UpstreamFormat::from_transformer_name(&e.transformer),
                    UpstreamFormat::OpenAiResponses | UpstreamFormat::OpenAiChat
                )
            })
            .collect();
        if filtered.is_empty() {
            return json_error(
                StatusCode::BAD_REQUEST,
                "Responses 入站(/v1/responses)无可用的 codex/openai 端点",
            );
        }
        filtered
    } else {
        enabled
    };

    let inbound_label = if inbound_openai {
        "openai"
    } else if inbound_responses {
        "responses"
    } else {
        "claude"
    };
    tracing::info!(
        path = %path,
        inbound = inbound_label,
        model = model.as_deref().unwrap_or("-"),
        stream = client_wants_stream,
        candidates = enabled.len(),
        "入站请求"
    );

    let resolution = resolver::resolve_endpoint(&hmap, model.as_deref(), &qmap, &enabled);
    if let Some(msg) = resolution.not_found {
        return json_error(StatusCode::BAD_REQUEST, &msg);
    }
    let use_specific = resolution.use_specific();
    let enabled = if use_specific {
        enabled
    } else {
        let filtered = filter_unavailable_by_test_status(&enabled);
        if filtered.len() < enabled.len() {
            tracing::info!(
                before = enabled.len(),
                after = filtered.len(),
                "路由过滤：跳过手动测试不可用的端点"
            );
        }
        filtered
    };
    // 模型可用性过滤：非显式端点时，仅在「声明了该请求模型」的端点间轮换/熔断（故障隔离）；
    // 无任一端点声明该模型则回退全量（向后兼容）。显式指定端点遵从用户意图，不过滤。
    let enabled: Vec<Endpoint> = if use_specific {
        enabled
    } else {
        // 模型过滤前日志：打印每个端点公布的模型
        if let Some(m) = model.as_deref() {
            tracing::debug!(model = m, candidates = enabled.len(), "模型过滤前候选端点");
            for ep in &enabled {
                let advertised = resolver::advertised_models(ep);
                tracing::debug!(
                    endpoint = %ep.name,
                    advertised_models = ?advertised,
                    locked_model = %ep.model,
                    models_count = ep.models.len(),
                    active_models_count = ep.active_models.len(),
                    "端点公布模型详情"
                );
            }
        }

        let any_declared = model
            .as_deref()
            .is_some_and(|m| any_endpoint_declares_model(&enabled, m));
        let filtered = resolver::filter_by_model(&enabled, model.as_deref());
        let filtered = resolver::order_candidates_for_model_mapping(
            &filtered,
            model.as_deref(),
            &st.model_mapping_strategy,
        );

        // 模型过滤后日志：汇总过滤结果
        if let Some(m) = model.as_deref() {
            let before_count = enabled.len();
            let after_count = filtered.len();
            if after_count < before_count {
                tracing::info!(
                    model = m,
                    before = before_count,
                    after = after_count,
                    filtered_out = before_count - after_count,
                    "模型过滤完成"
                );
                let filtered_names: Vec<String> = enabled
                    .iter()
                    .filter(|e| !filtered.iter().any(|f| f.name == e.name))
                    .map(|e| e.name.clone())
                    .collect();
                if !filtered_names.is_empty() {
                    tracing::debug!(
                        filtered_endpoints = ?filtered_names,
                        "以下端点未声明该模型，已过滤"
                    );
                }
            } else if after_count == before_count && before_count > 0 && any_declared {
                tracing::debug!(
                    model = m,
                    candidates = after_count,
                    "模型过滤：全部候选均声明该模型"
                );
            } else if after_count == before_count && before_count > 0 {
                tracing::debug!(
                    model = m,
                    candidates = after_count,
                    "模型过滤：无端点声明该模型，回退全量（向后兼容）"
                );
            }
        }

        filtered
    };
    // 熔断选路：非显式端点时过滤掉未到期的 Open 端点（全 Open 则返回空，由下方守卫返回 502）。
    // 显式指定端点绕过熔断（用户意图优先），但结果仍计入熔断。
    let enabled_before_breaker = enabled.clone();
    let (enabled, gate): (Vec<Endpoint>, bool) = if use_specific {
        (enabled, false)
    } else {
        let cands = circuit_breaker::select_candidates(&enabled, &st.breakers, Instant::now());
        // 候选变少 → 剔除了 Open，存在可用子集，需对候选取许可（半开单探测）；
        // 数量不变 → 无 Open 或全 Open 兜底，均不 gate。
        let gate = cands.len() < enabled.len();

        // 熔断选路后日志
        if gate {
            let filtered_names: Vec<String> = enabled_before_breaker
                .iter()
                .filter(|e| !cands.iter().any(|c| c.name == e.name))
                .map(|e| e.name.clone())
                .collect();
            tracing::info!(
                before = enabled_before_breaker.len(),
                after = cands.len(),
                filtered_out = enabled_before_breaker.len() - cands.len(),
                open_endpoints = ?filtered_names,
                "熔断过滤完成（存在 Open 端点）"
            );
        } else if !enabled_before_breaker.is_empty() {
            tracing::debug!(candidates = cands.len(), "熔断过滤：无 Open 端点或全 Open");
        }

        (cands, gate)
    };
    let breaker_exhausted =
        !use_specific && !enabled_before_breaker.is_empty() && enabled.is_empty();
    // 熔断后二次模型过滤：当熔断器保留了兜底候选（全 Open）时，再次按模型支持性过滤，避免误伤不支持该模型的端点。
    // 此过滤仅在「模型过滤阶段已回退全量」时生效（即 enabled 经过模型过滤后未减少，说明无端点声明该模型）。
    let enabled: Vec<Endpoint> = if !use_specific && model.is_some() {
        let before_count = enabled.len();
        let model_filtered: Vec<Endpoint> = enabled
            .into_iter()
            .filter(|ep| {
                let advertised = resolver::advertised_models(ep);
                if advertised.is_empty() {
                    // 端点未公布任何模型（旧端点向后兼容）→ 保留
                    true
                } else {
                    // 端点公布了模型 → 仅当声明了请求模型时保留
                    advertised.iter().any(|am| {
                        am.trim()
                            .eq_ignore_ascii_case(model.as_deref().unwrap().trim())
                    })
                }
            })
            .collect();
        let after_count = model_filtered.len();
        if after_count < before_count {
            let filtered_names: Vec<String> = enabled_before_breaker
                .iter()
                .filter(|e| !model_filtered.iter().any(|f| f.name == e.name))
                .map(|e| e.name.clone())
                .collect();
            if after_count == 0 {
                tracing::warn!(
                    model = model.as_deref().unwrap_or("-"),
                    filtered_endpoints = ?filtered_names,
                    "所有候选端点均不支持请求的模型"
                );
            } else {
                tracing::debug!(
                    model = model.as_deref().unwrap_or("-"),
                    before = before_count,
                    after = after_count,
                    "熔断兜底后按模型过滤"
                );
            }
        }
        model_filtered
    } else {
        enabled
    };
    let n = enabled.len();
    if n == 0 {
        let msg = empty_candidates_message(model.as_deref(), breaker_exhausted);
        return json_error(StatusCode::BAD_GATEWAY, &msg);
    }
    let max = if st.max_retries > 0 {
        st.max_retries as usize
    } else if use_specific {
        3
    } else {
        rotation::max_retries(n).max(1)
    };

    let mut attempts_on_current = 0u32;
    let mut last_err = String::new();
    let mut last_endpoint = String::new();
    let mut last_status: Option<u16> = None;
    let mut last_error_body: Option<String> = None;
    let mut last_request_meta: Option<RequestMeta> = None;
    // 最后一次实际尝试的出站路径：全部失败兜底记录时填入，避免前端按入站格式误推断。
    let mut last_upstream_path = String::new();
    // thinking 签名整流一次性标志：命中后清洗重试，仅一次，防死循环。
    let mut sig_rectified = false;

    for _ in 0..max {
        let ep: Endpoint = if let Some(ref e) = resolution.endpoint {
            e.clone()
        } else {
            let idx = st.rotation.current_index(n).unwrap_or(0);
            enabled[idx].clone()
        };
        st.set_current(&ep.name);
        last_endpoint = ep.name.clone();

        // 熔断许可：gate 时对候选取许可（半开同一时刻仅 1 个探测）；拒绝则跳到下一个端点。
        let used_permit = if gate {
            let allow = st.breakers.allow_request(&ep.name, Instant::now());
            if !allow.allowed {
                st.rotation.advance(n);
                continue;
            }
            allow.used_half_open_permit
        } else {
            false
        };

        let format = UpstreamFormat::from_transformer_name(&ep.transformer);
        // 出站模型解析：入站名命中映射 → 出站名；否则锁定 model；否则透传（空串）。
        let outbound_model = resolver::resolve_outbound(&ep, model.as_deref()).unwrap_or_default();
        // 转换场景（互斥）：Claude 入站+OpenAI 端点 → Claude↔Chat；Responses 入站+OpenAI 端点 → Responses↔Chat；
        // Responses 入站+codex 端点 → 透传（仅改 model）；其余（OpenAI 入站透传、Claude 直通）不转换。
        let needs_transform =
            !inbound_openai && !inbound_responses && matches!(format, UpstreamFormat::OpenAiChat);
        let responses_to_chat =
            should_route_responses_via_chat(inbound_responses, client_wants_stream, format);
        let attempt_body: Bytes = if needs_transform {
            // Claude → OpenAI（transform_request 内部按出站模型覆盖，空则透传客户端 model）
            match &body_json {
                Some(cj) => get_transformer(format)
                    .transform_request(cj, Some(&outbound_model))
                    .ok()
                    .and_then(|v| serde_json::to_vec(&v).ok())
                    .map(Bytes::from)
                    .unwrap_or_else(|| body.clone()),
                None => body.clone(),
            }
        } else if responses_to_chat {
            // Responses → Chat（responses_request_to_chat 内部按出站模型覆盖，空则透传客户端 model）
            match &body_json {
                Some(cj) => {
                    serde_json::to_vec(&responses_request_to_chat(cj, Some(&outbound_model)))
                        .map(Bytes::from)
                        .unwrap_or_else(|_| body.clone())
                }
                None => body.clone(),
            }
        } else if !outbound_model.is_empty() {
            // 直通场景：映射/锁定的出站模型覆盖请求体 model 后重新序列化
            match &body_json {
                Some(cj) => {
                    let mut v = cj.clone();
                    if let Some(o) = v.as_object_mut() {
                        o.insert("model".to_string(), Value::String(outbound_model.clone()));
                    }
                    serde_json::to_vec(&v)
                        .map(Bytes::from)
                        .unwrap_or_else(|_| body.clone())
                }
                None => body.clone(),
            }
        } else if sig_rectified {
            // 已签名整流：从整流后的 body_json 重新序列化（直通空 model 分支否则会回退用原始未整流字节）
            match &body_json {
                Some(cj) => serde_json::to_vec(cj)
                    .map(Bytes::from)
                    .unwrap_or_else(|_| body.clone()),
                None => body.clone(),
            }
        } else {
            body.clone()
        };
        let upstream_path = if needs_transform || responses_to_chat {
            "/v1/chat/completions"
        } else {
            path.as_str()
        };
        let upstream_url = format!("{}{}", ep.api_url.trim_end_matches('/'), upstream_path);
        maybe_learn_client_ua(&st, &ep, &headers);
        let forward_headers = build_forward_headers(&st, &ep, &headers);
        last_upstream_path = upstream_path.to_string();
        let route_mode = if responses_to_chat {
            "responses->chat"
        } else if needs_transform {
            "claude->openai"
        } else {
            "passthrough"
        };

        // 检查选中端点是否真正声明了请求模型
        let model_declared = if let Some(ref m) = model {
            resolver::advertised_models(&ep)
                .iter()
                .any(|am| am.trim().eq_ignore_ascii_case(m.trim()))
        } else {
            true // 无模型请求视为匹配
        };

        tracing::info!(
            endpoint = %ep.name,
            transformer = %ep.transformer,
            mode = route_mode,
            outbound_model = %outbound_model,
            upstream_path,
            model_declared,
            "转发上游"
        );

        // 如果端点未声明该模型，记录警告（可能是配置错误或回退逻辑）
        if !model_declared {
            let advertised = resolver::advertised_models(&ep);
            tracing::warn!(
                endpoint = %ep.name,
                requested_model = model.as_deref().unwrap_or("-"),
                advertised = ?advertised,
                "警告：选中端点未声明请求模型（可能是配置错误或回退逻辑）"
            );
        }

        let token = st.active.start(&ep.name);
        let meta_template = RequestMeta {
            endpoint: ep.name.clone(),
            model: model.clone(),
            inbound_format: (if inbound_openai {
                "openai"
            } else if inbound_responses {
                "responses"
            } else {
                "claude"
            })
            .to_string(),
            upstream_url: ep.api_url.clone(),
            inbound_path: path.clone(),
            upstream_path: upstream_path.to_string(),
            started_ms,
            first_byte_ms: None,
            actual_model: None,
            trace: RequestTrace {
                received_request: received_request_trace.clone(),
                forward_request: trace_capture::request_stage_from_pairs(
                    &method,
                    upstream_url.clone(),
                    &forward_headers,
                    &attempt_body,
                ),
                ..empty_trace()
            },
        };
        let result = tokio::select! {
            r = send_upstream(&st, &ep, &method, upstream_path, &headers, &attempt_body) => Some(r),
            _ = token.cancelled() => None,
        };
        st.active.finish(&ep.name);
        last_request_meta = Some(meta_template.clone());

        match result {
            // 被手动切换取消 → 切到下一个
            None => {
                st.breakers.record_neutral(&ep.name, used_permit);
                last_err = "请求被取消".to_string();
                if !use_specific {
                    st.rotation.advance(n);
                }
                attempts_on_current = 0;
            }
            Some(Ok(resp)) => {
                let status = resp.status().as_u16();
                tracing::info!(endpoint = %ep.name, status, "上游响应");
                // 实际(出站)模型：改写后的 outbound_model 非空且与入站不同（ci）才记录，供前端展示"实际模型"。
                let requested = model.as_deref().unwrap_or("");
                let actual_model = if !outbound_model.is_empty()
                    && !outbound_model.eq_ignore_ascii_case(requested)
                {
                    Some(outbound_model.clone())
                } else {
                    None
                };
                let mut meta = meta_template.clone();
                meta.first_byte_ms = Some(chrono::Utc::now().timestamp_millis() - started_ms);
                meta.actual_model = actual_model;
                if status == 200 {
                    if responses_to_chat
                        && client_wants_stream
                        && !response_is_event_stream(resp.headers())
                    {
                        let upstream_trace_headers = trace_capture::capture_headers(resp.headers());
                        let err_bytes = resp.bytes().await.unwrap_or_default();
                        let err_text = String::from_utf8_lossy(&err_bytes);
                        let trace_body = trace_capture::capture_body(&err_bytes);
                        set_upstream_trace(&mut meta, status, upstream_trace_headers, trace_body);
                        last_request_meta = Some(meta);
                        last_status = Some(status);
                        last_error_body = error_body_from_bytes(&err_bytes);
                        last_err = format!(
                            "上游未返回 SSE 流，无法转换为 Responses 流式响应: {}",
                            truncate_error_body(&err_text)
                        );
                        if st.breakers.record_failure(
                            &ep.name,
                            used_permit,
                            Instant::now(),
                            "upstream returned non-SSE body for stream request",
                        ) {
                            st.stats.emit_health_changed();
                        }
                        if !use_specific {
                            st.rotation.advance(n);
                        }
                        attempts_on_current = 0;
                        continue;
                    }
                    // 成功：闭合熔断（半开恢复时回传许可）；转换则通知前端
                    if st.breakers.record_success(&ep.name, used_permit) {
                        st.stats.emit_health_changed();
                    }
                    // 真实 token 由各响应处理函数解析上游 usage 后记录
                    let stats = st.stats.clone();
                    if responses_to_chat {
                        // Responses 入站 + OpenAI 端点：上游 Chat 响应转回 Responses。
                        return if client_wants_stream {
                            stream_responses_from_chat(resp, stats, meta)
                        } else {
                            buffered_responses_from_chat(resp, stats, meta).await
                        };
                    }
                    return match (needs_transform, client_wants_stream) {
                        (true, true) => stream_transform_response(resp, stats, meta),
                        (true, false) => transform_buffered_response(resp, stats, meta).await,
                        (false, true) => relay_stream_response(resp, st.clone(), meta, format),
                        (false, false) => relay_buffered_response(resp, stats, meta, format).await,
                    };
                }
                // 非 200：按状态码归类上报熔断（客户端错误中性、其余计入失败）
                match rotation::categorize_status(status) {
                    rotation::Outcome::NonRetryable => {
                        st.breakers.record_neutral(&ep.name, used_permit)
                    }
                    rotation::Outcome::Retryable => {
                        if st.breakers.record_failure(
                            &ep.name,
                            used_permit,
                            Instant::now(),
                            &format!("HTTP {status}"),
                        ) {
                            st.stats.emit_health_changed();
                        }
                    }
                }
                if !rotation::should_retry_status(status) {
                    // 读错误体：Responses→Chat reasoning_effort 降级 / thinking 签名整流（透明重试）。
                    let may_downgrade_effort = responses_to_chat && st.reasoning_effort_fallback;
                    let may_rectify_sig = st.rectifier_config.enabled && !sig_rectified;
                    let resp_headers = copy_response_headers(&resp);
                    let upstream_trace_headers = trace_capture::capture_headers(resp.headers());
                    let err_bytes = resp.bytes().await.unwrap_or_default();
                    let err_text = String::from_utf8_lossy(&err_bytes);

                    if may_downgrade_effort || may_rectify_sig {
                        if may_downgrade_effort && is_unsupported_reasoning_effort_error(&err_text)
                        {
                            if let Some(cj) = body_json.as_mut() {
                                if downgrade_reasoning_effort_in_responses(cj) {
                                    tracing::info!(
                                        endpoint = %ep.name,
                                        "reasoning.effort 不被上游接受，已降级并重试"
                                    );
                                    continue;
                                }
                            }
                        }

                        if may_rectify_sig
                            && should_rectify_thinking_signature(
                                Some(&err_text),
                                &st.rectifier_config,
                            )
                        {
                            if let Some(cj) = body_json.as_mut() {
                                if rectify_anthropic_request(cj).applied {
                                    // 清洗成功：不计失败、不前进轮换，下一轮用整流后的 body_json 重试同端点
                                    sig_rectified = true;
                                    continue;
                                }
                            }
                        }

                        // 未命中 / 无可整流内容：用缓冲错误体原样回传
                        let trace_body = trace_capture::capture_body(&err_bytes);
                        set_upstream_trace(
                            &mut meta,
                            status,
                            upstream_trace_headers.clone(),
                            trace_body.clone(),
                        );
                        set_response_trace(
                            &mut meta,
                            status,
                            trace_capture::capture_headers(&resp_headers),
                            trace_body,
                        );
                        st.stats.record(meta.into_record_with_error_body(
                            Some(status as i64),
                            status >= 400,
                            usage::TokenUsage::default(),
                            error_body_from_bytes(&err_bytes),
                        ));
                        return relay_buffered_error(status, resp_headers, err_bytes);
                    }
                    // 最终非重试状态（400/401）缓冲回传，便于记录上游错误体。
                    let trace_body = trace_capture::capture_body(&err_bytes);
                    set_upstream_trace(
                        &mut meta,
                        status,
                        upstream_trace_headers,
                        trace_body.clone(),
                    );
                    set_response_trace(
                        &mut meta,
                        status,
                        trace_capture::capture_headers(&resp_headers),
                        trace_body,
                    );
                    st.stats.record(meta.into_record_with_error_body(
                        Some(status as i64),
                        status >= 400,
                        usage::TokenUsage::default(),
                        error_body_from_bytes(&err_bytes),
                    ));
                    return relay_buffered_error(status, resp_headers, err_bytes);
                }
                last_err = format!("上游返回 {status}");
                last_status = Some(status);
                let upstream_trace_headers = trace_capture::capture_headers(resp.headers());
                let err_bytes = resp.bytes().await.unwrap_or_default();
                last_error_body = error_body_from_bytes(&err_bytes);
                if let Some(meta) = last_request_meta.as_mut() {
                    set_upstream_trace(
                        meta,
                        status,
                        upstream_trace_headers,
                        trace_capture::capture_body(&err_bytes),
                    );
                }
                attempts_on_current += 1;
                if attempts_on_current >= rotation::CONSECUTIVE_FAIL_SWITCH && !use_specific {
                    st.rotation.advance(n);
                    attempts_on_current = 0;
                }
            }
            Some(Err(e)) => {
                let msg = e.to_string();
                // 网络错误计入熔断（Retryable）；转换则通知前端
                if st
                    .breakers
                    .record_failure(&ep.name, used_permit, Instant::now(), &msg)
                {
                    st.stats.emit_health_changed();
                }
                last_err = msg.clone();
                if rotation::is_transient_network_error(&msg) {
                    // 瞬时错误：延迟后重试同一端点
                    tokio::time::sleep(rotation::TRANSIENT_RETRY_DELAY).await;
                    attempts_on_current = 0;
                } else {
                    attempts_on_current += 1;
                    if attempts_on_current >= rotation::CONSECUTIVE_FAIL_SWITCH && !use_specific {
                        st.rotation.advance(n);
                        attempts_on_current = 0;
                    }
                }
            }
        }
    }

    let diagnostic_message = last_status
        .and_then(|status| {
            last_error_body.as_deref().map(|body| {
                diagnose_upstream_error(status, body).format_for_proxy(
                    &last_endpoint,
                    status,
                    model.as_deref(),
                )
            })
        })
        .unwrap_or_else(|| format!("所有端点均失败: {last_err}"));
    let diagnostic_json = serde_json::to_string(&diagnostic_message)
        .unwrap_or_else(|_| "\"所有端点均失败，且错误信息无法序列化\"".to_string());
    let last_upstream_trace_body = last_error_body
        .clone()
        .or_else(|| trace_capture::capture_text_body(&diagnostic_message));

    if let Some(mut meta) = last_request_meta {
        meta.trace.received_forwarded_request = trace_capture::response_stage(
            meta.trace
                .forward_request
                .url
                .clone()
                .unwrap_or_else(|| meta.upstream_path.clone()),
            last_status.map(i64::from),
            Vec::new(),
            last_upstream_trace_body.clone(),
        );
        meta.trace.response_request = trace_capture::response_stage(
            meta.inbound_path.clone(),
            Some(StatusCode::BAD_GATEWAY.as_u16() as i64),
            trace_capture::json_headers(),
            trace_capture::capture_text_body(&format!(
                r#"{{"error":{{"type":"proxy_error","message":{diagnostic_json}}}}}"#
            )),
        );
        st.stats.record(meta.into_record_with_error_body(
            Some(StatusCode::BAD_GATEWAY.as_u16() as i64),
            true,
            usage::TokenUsage::default(),
            last_error_body,
        ));
    } else if !last_endpoint.is_empty() {
        st.stats.record(RequestRecord {
            endpoint_name: last_endpoint.clone(),
            model: model.clone(),
            inbound_format: inbound_label.to_string(),
            upstream_url: String::new(),
            inbound_path: path.clone(),
            upstream_path: last_upstream_path.clone(),
            status_code: Some(StatusCode::BAD_GATEWAY.as_u16() as i64),
            is_error: true,
            usage: usage::TokenUsage::default(),
            duration_ms: Some(chrono::Utc::now().timestamp_millis() - started_ms),
            first_byte_ms: None,
            actual_model: None,
            error_body: last_error_body,
            trace: Some(RequestTrace {
                received_request: received_request_trace,
                forward_request: RequestTraceStage {
                    method: Some(method.to_string()),
                    url: if last_upstream_path.is_empty() {
                        None
                    } else {
                        Some(last_upstream_path.clone())
                    },
                    status_code: None,
                    headers: Vec::new(),
                    body: None,
                },
                received_forwarded_request: trace_capture::response_stage(
                    last_upstream_path.clone(),
                    last_status.map(i64::from),
                    Vec::new(),
                    last_upstream_trace_body,
                ),
                response_request: trace_capture::response_stage(
                    path.clone(),
                    Some(StatusCode::BAD_GATEWAY.as_u16() as i64),
                    trace_capture::json_headers(),
                    trace_capture::capture_text_body(&format!(
                        r#"{{"error":{{"type":"proxy_error","message":{diagnostic_json}}}}}"#
                    )),
                ),
            }),
        });
    }
    json_error(StatusCode::BAD_GATEWAY, &diagnostic_message)
}

async fn send_upstream(
    st: &ProxyState,
    ep: &Endpoint,
    method: &Method,
    upstream_path: &str,
    headers: &HeaderMap,
    body: &Bytes,
) -> reqwest::Result<reqwest::Response> {
    let base = ep.api_url.trim_end_matches('/');
    let url = format!("{base}{upstream_path}");
    let rmethod =
        reqwest::Method::from_bytes(method.as_str().as_bytes()).unwrap_or(reqwest::Method::POST);

    // 是否走代理：端点 use_proxy 或全局 proxy_enabled。want 但无可用代理 client 时 warn 后回落直连（不静默）。
    let want_proxy = ep.use_proxy || st.proxy_enabled;
    let client = if want_proxy {
        match st.proxy_client.as_ref() {
            Some(c) => c,
            None => {
                tracing::warn!(
                    endpoint = %ep.name,
                    "已请求经代理出网，但无可用代理 client（代理地址为空/无效），回落直连"
                );
                &st.client
            }
        }
    } else {
        &st.client
    };
    let mut rb = client.request(rmethod, url);

    // 伪装 UA：按上游格式取配置；非空则覆盖客户端 UA，为空则纯透传（客户端 UA 随下方头部复制原样转发）
    let ua_format = UpstreamFormat::from_transformer_name(&ep.transformer);
    let ua_override = match ua_format {
        UpstreamFormat::OpenAiChat | UpstreamFormat::OpenAiResponses => current_openai_ua(st),
        UpstreamFormat::Claude => current_claude_ua(st),
    };
    let ua_override = ua_override.trim().to_string();
    let override_ua = !ua_override.is_empty();
    let add_codex_originator = override_ua
        && ua_override.starts_with(ua::CODEX_ORIGINATOR)
        && !headers.contains_key("originator");

    // 复制客户端头部（剔除 Host / Content-Length / Accept-Encoding / 客户端凭证 / 控制头；
    // 仅在配置了伪装 UA 时剔除客户端 user-agent，否则原样透传客户端 UA）
    for (k, v) in headers.iter() {
        let kn = k.as_str().to_ascii_lowercase();
        if kn == "host"
            || kn == "content-length"
            || kn == "accept-encoding"
            || kn == "authorization"
            || kn == "x-api-key"
            || kn == resolver::ENDPOINT_HEADER
            || kn == resolver::ENDPOINT_HEADER_ALT
            || (override_ua && kn == "user-agent")
        {
            continue;
        }
        if let Ok(val) = v.to_str() {
            rb = rb.header(k.as_str(), val);
        }
    }

    // 配置了伪装 UA 才覆盖；未配置时客户端 UA 已在上面原样透传
    if override_ua {
        rb = rb.header("user-agent", ua_override);
    }
    if add_codex_originator {
        rb = rb.header("originator", ua::CODEX_ORIGINATOR);
    }

    // 附加鉴权头（按 transformer / auth_mode）
    let key = ep.api_key.as_str();
    if !key.is_empty() {
        match ep.transformer.as_str() {
            "openai" | "openai2" | "openai_chat" => {
                rb = rb.header("authorization", format!("Bearer {key}"));
            }
            _ => {
                rb = rb
                    .header("x-api-key", key)
                    .header("authorization", format!("Bearer {key}"));
            }
        }
    }

    rb.body(body.clone()).send().await
}

/// 复制上游响应头（剔除逐跳头）。
fn copy_response_headers(resp: &reqwest::Response) -> HeaderMap {
    let mut out = HeaderMap::new();
    for (k, v) in resp.headers().iter() {
        let kn = k.as_str().to_ascii_lowercase();
        if kn == "content-length" || kn == "transfer-encoding" || kn == "connection" {
            continue;
        }
        if let (Ok(name), Ok(val)) = (
            HeaderName::from_bytes(k.as_str().as_bytes()),
            HeaderValue::from_bytes(v.as_bytes()),
        ) {
            out.insert(name, val);
        }
    }
    out
}

/// 由已缓冲的错误字节重建响应（错误体被读取用于整流匹配后，无法再流式直通）。
fn relay_buffered_error(status_code: u16, headers: HeaderMap, bytes: Bytes) -> Response {
    let status = StatusCode::from_u16(status_code).unwrap_or(StatusCode::BAD_GATEWAY);
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = status;
    *response.headers_mut() = headers;
    response
}

/// 非流式直通：缓冲响应体 → 按上游格式解析真实 usage 记录统计 → 原样回传。
async fn relay_buffered_response(
    resp: reqwest::Response,
    stats: Arc<StatsAggregator>,
    mut meta: RequestMeta,
    format: UpstreamFormat,
) -> Response {
    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let upstream_trace_headers = trace_capture::capture_headers(resp.headers());
    let out = copy_response_headers(&resp);
    let bytes = match resp.bytes().await {
        Ok(b) => b,
        Err(e) => return json_error(StatusCode::BAD_GATEWAY, &format!("读取上游响应失败: {e}")),
    };
    let trace_body = trace_capture::capture_body(&bytes);
    set_upstream_trace(
        &mut meta,
        status.as_u16(),
        upstream_trace_headers,
        trace_body.clone(),
    );
    set_response_trace(
        &mut meta,
        status.as_u16(),
        trace_capture::capture_headers(&out),
        trace_body,
    );
    let tu = serde_json::from_slice::<Value>(&bytes)
        .map(|j| usage::from_response(&j, format))
        .unwrap_or_default();
    stats.record(meta.into_record(Some(status.as_u16() as i64), false, tu));
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = status;
    *response.headers_mut() = out;
    response
}

/// 流式直通：转发原始 SSE 字节同时累积真实 usage，流结束记录统计。
fn relay_stream_response(
    resp: reqwest::Response,
    st: Arc<ProxyState>,
    meta: RequestMeta,
    format: UpstreamFormat,
) -> Response {
    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let upstream_trace_headers = trace_capture::capture_headers(resp.headers());
    let out = copy_response_headers(&resp);
    let response_trace_headers = trace_capture::capture_headers(&out);
    let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(32);
    tokio::spawn(async move {
        let mut meta = meta;
        let endpoint_name = meta.endpoint.clone();
        let mut acc = usage::UsageAccumulator::new(format);
        let mut upstream_preview = trace_capture::TraceBodyCapture::default();
        let mut response_preview = trace_capture::TraceBodyCapture::default();
        let mut stream = resp.bytes_stream();
        let mut first = true;
        let mut read_error: Option<String> = None;
        let mut saw_responses_completed = !matches!(format, UpstreamFormat::OpenAiResponses);
        while let Some(item) = stream.next().await {
            let chunk = match item {
                Ok(c) => c,
                Err(e) => {
                    read_error = Some(e.to_string());
                    break;
                }
            };
            if first {
                // 首个内容分片到达 → 更精确的首字延迟，覆盖响应头时刻。
                meta.first_byte_ms = Some(chrono::Utc::now().timestamp_millis() - meta.started_ms);
                first = false;
            }
            acc.feed(&chunk);
            if matches!(format, UpstreamFormat::OpenAiResponses)
                && String::from_utf8_lossy(&chunk).contains("response.completed")
            {
                saw_responses_completed = true;
            }
            upstream_preview.push(&chunk);
            response_preview.push(&chunk);
            if tx.send(Ok(chunk)).await.is_err() {
                break;
            }
        }
        set_upstream_trace(
            &mut meta,
            status.as_u16(),
            upstream_trace_headers,
            upstream_preview.finish(),
        );
        set_response_trace(
            &mut meta,
            status.as_u16(),
            response_trace_headers,
            response_preview.finish(),
        );
        let tu = acc.finish();
        let error_body = responses_stream_error_body(
            format,
            status.as_u16(),
            read_error.as_deref(),
            saw_responses_completed,
        );
        if let Some(error_body) = error_body.as_deref() {
            if st
                .breakers
                .record_failure(&endpoint_name, false, Instant::now(), error_body)
            {
                st.stats.emit_health_changed();
            }
        }
        st.stats.record(meta.into_record_with_error_body(
            Some(status.as_u16() as i64),
            error_body.is_some(),
            tu,
            error_body,
        ));
    });
    let body = Body::from_stream(ReceiverStream::new(rx));
    let mut response = Response::new(body);
    *response.status_mut() = status;
    *response.headers_mut() = out;
    response
}

fn responses_stream_error_body(
    format: UpstreamFormat,
    status: u16,
    read_error: Option<&str>,
    saw_responses_completed: bool,
) -> Option<String> {
    if !matches!(format, UpstreamFormat::OpenAiResponses) || status != 200 {
        return None;
    }
    if let Some(read_error) = read_error {
        return Some(format!(
            "上游 /v1/responses 流式响应读取失败：{read_error}。这通常会导致客户端看到 stream disconnected before completion；请降低该端点排序或改用 openai 类型走 /v1/chat/completions 转换链路。"
        ));
    }
    if !saw_responses_completed {
        return Some(
            "上游 /v1/responses 流式响应已结束，但没有发送 response.completed 结束事件。该端点不满足 Codex Responses 流式协议，可能会导致 Codex 提示 stream disconnected before completion；请降低该端点排序、禁用该模型，或改用 openai 类型走 /v1/chat/completions 转换链路。"
                .to_string(),
        );
    }
    None
}

/// 非流式 OpenAI 响应 → 缓冲后转换为 Claude JSON 回传；记录真实 usage。
async fn transform_buffered_response(
    resp: reqwest::Response,
    stats: Arc<StatsAggregator>,
    mut meta: RequestMeta,
) -> Response {
    let upstream_trace_headers = trace_capture::capture_headers(resp.headers());
    match resp.text().await {
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Ok(openai) => {
                let final_body = serde_json::to_string_pretty(&openai_response_to_claude(&openai))
                    .unwrap_or_else(|_| text.clone());
                set_upstream_trace(
                    &mut meta,
                    200,
                    upstream_trace_headers,
                    trace_capture::capture_text_body(&text),
                );
                set_response_trace(
                    &mut meta,
                    200,
                    trace_capture::json_headers(),
                    trace_capture::capture_text_body(&final_body),
                );
                let tu = usage::from_response(&openai, UpstreamFormat::OpenAiChat);
                stats.record(meta.into_record(Some(200), false, tu));
                (
                    StatusCode::OK,
                    axum::Json(openai_response_to_claude(&openai)),
                )
                    .into_response()
            }
            Err(_) => json_error(StatusCode::BAD_GATEWAY, "上游响应解析失败"),
        },
        Err(e) => json_error(StatusCode::BAD_GATEWAY, &format!("读取上游响应失败: {e}")),
    }
}

/// 流式 OpenAI SSE → 边解析边转换为 Claude SSE 事件流回传；流结束记录真实 usage。
fn stream_transform_response(
    resp: reqwest::Response,
    stats: Arc<StatsAggregator>,
    meta: RequestMeta,
) -> Response {
    let upstream_trace_headers = trace_capture::capture_headers(resp.headers());
    let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(32);
    tokio::spawn(async move {
        let mut meta = meta;
        let mut converter = StreamConverter::new(meta.model.clone().unwrap_or_default(), 0);
        let mut upstream_preview = trace_capture::TraceBodyCapture::default();
        let mut response_preview = trace_capture::TraceBodyCapture::default();
        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        let mut first = true;
        while let Some(item) = stream.next().await {
            let chunk = match item {
                Ok(c) => c,
                Err(_) => break,
            };
            if first {
                meta.first_byte_ms = Some(chrono::Utc::now().timestamp_millis() - meta.started_ms);
                first = false;
            }
            upstream_preview.push(&chunk);
            buf.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(nl) = buf.find('\n') {
                let line: String = buf[..nl].to_string();
                buf.drain(..=nl);
                let line = line.trim();
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    for ev in converter.finish() {
                        let chunk = Bytes::from(ev);
                        response_preview.push(&chunk);
                        let _ = tx.send(Ok(chunk)).await;
                    }
                } else if !data.is_empty() {
                    if let Ok(j) = serde_json::from_str::<Value>(data) {
                        for ev in converter.process_chunk(&j) {
                            let chunk = Bytes::from(ev);
                            response_preview.push(&chunk);
                            let _ = tx.send(Ok(chunk)).await;
                        }
                    }
                }
            }
        }
        // 上游未发 [DONE] 时兜底收尾（finish 幂等）
        for ev in converter.finish() {
            let chunk = Bytes::from(ev);
            response_preview.push(&chunk);
            let _ = tx.send(Ok(chunk)).await;
        }
        set_upstream_trace(
            &mut meta,
            200,
            upstream_trace_headers,
            upstream_preview.finish(),
        );
        set_response_trace(
            &mut meta,
            200,
            trace_capture::sse_headers(),
            response_preview.finish(),
        );
        let (input, output, cache_creation, cache_read) = converter.usage();
        let tu = usage::TokenUsage {
            input,
            output,
            cache_creation,
            cache_read,
        };
        stats.record(meta.into_record(Some(200), false, tu));
    });

    let body = Body::from_stream(ReceiverStream::new(rx));
    let mut response = Response::new(body);
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::{
        empty_candidates_message, error_body_from_bytes, filter_unavailable_by_test_status,
        learned_ua_key, responses_stream_error_body, should_route_responses_via_chat,
        truncate_error_body,
    };
    use crate::models::endpoint::Endpoint;
    use crate::modules::transform::transformer::UpstreamFormat;
    use axum::body::Bytes;

    #[test]
    fn empty_candidates_message_prefers_breaker_reason_over_model() {
        let msg = empty_candidates_message(Some("gpt-5.5"), true);
        assert_eq!(msg, "所有候选端点均熔断或无可用端点");
    }

    #[test]
    fn empty_candidates_message_reports_unsupported_model_when_not_breaker_exhausted() {
        let msg = empty_candidates_message(Some("gpt-5.5"), false);
        assert_eq!(msg, "所有候选端点均不支持模型 'gpt-5.5'");
    }

    #[test]
    fn error_body_from_bytes_keeps_small_body() {
        let body = error_body_from_bytes(&Bytes::from_static(br#"{"error":"x"}"#));
        assert_eq!(body.as_deref(), Some(r#"{"error":"x"}"#));
    }

    #[test]
    fn responses_stream_routes_only_openai_endpoints_via_chat() {
        assert!(!should_route_responses_via_chat(
            true,
            true,
            UpstreamFormat::OpenAiResponses
        ));
        assert!(should_route_responses_via_chat(
            true,
            true,
            UpstreamFormat::OpenAiChat
        ));
        assert!(!should_route_responses_via_chat(
            true,
            false,
            UpstreamFormat::OpenAiResponses
        ));
        assert!(!should_route_responses_via_chat(
            false,
            true,
            UpstreamFormat::OpenAiResponses
        ));
    }

    #[test]
    fn learned_ua_updates_default_openai_but_not_custom_value() {
        let incoming = "codex_cli_rs/9.9.9 (windows; x86_64) vscode/1.99.0";
        assert_eq!(
            learned_ua_key(
                UpstreamFormat::OpenAiResponses,
                incoming,
                "codex_cli_rs/0.114.0 (windows; x86_64) vscode/1.111.0",
            ),
            Some("openaiUa")
        );
        assert_eq!(
            learned_ua_key(UpstreamFormat::OpenAiResponses, incoming, "my-custom-agent"),
            None
        );
    }

    #[test]
    fn learned_ua_updates_default_claude_but_not_custom_value() {
        let incoming = "claude-cli/9.9.9 (external, sdk-cli)";
        assert_eq!(
            learned_ua_key(
                UpstreamFormat::Claude,
                incoming,
                "claude-cli/2.1.185 (external, sdk-cli)",
            ),
            Some("claudeCliUa")
        );
        assert_eq!(
            learned_ua_key(UpstreamFormat::Claude, incoming, "my-agent"),
            None
        );
    }

    #[test]
    fn codex_stream_without_completed_is_recorded_as_error() {
        let message =
            responses_stream_error_body(UpstreamFormat::OpenAiResponses, 200, None, false)
                .expect("missing completed event should be an error");

        assert!(message.contains("response.completed"));
        assert!(message.contains("stream disconnected before completion"));
        assert!(message.contains("openai"));
    }

    #[test]
    fn codex_stream_read_error_is_recorded_as_error() {
        let message = responses_stream_error_body(
            UpstreamFormat::OpenAiResponses,
            200,
            Some("connection reset"),
            false,
        )
        .expect("stream read errors should be recorded");

        assert!(message.contains("读取失败"));
        assert!(message.contains("connection reset"));
    }

    #[test]
    fn completed_codex_stream_and_chat_stream_are_not_recorded_as_errors() {
        assert!(
            responses_stream_error_body(UpstreamFormat::OpenAiResponses, 200, None, true).is_none()
        );
        assert!(
            responses_stream_error_body(UpstreamFormat::OpenAiChat, 200, None, false).is_none()
        );
    }

    fn endpoint_with_test_status(name: &str, status: &str) -> Endpoint {
        Endpoint {
            id: 1,
            name: name.into(),
            api_url: "https://example.com".into(),
            api_key: String::new(),
            auth_mode: "api_key".into(),
            enabled: true,
            use_proxy: false,
            transformer: "codex".into(),
            model: String::new(),
            models: Vec::new(),
            active_models: Vec::new(),
            model_mappings: Vec::new(),
            balance_query: Default::default(),
            remark: String::new(),
            sort_order: 0,
            test_status: status.into(),
            created_at: String::new(),
            updated_at: String::new(),
        }
    }

    #[test]
    fn route_filter_skips_unavailable_test_status_when_possible() {
        let endpoints = vec![
            endpoint_with_test_status("bad", "unavailable"),
            endpoint_with_test_status("unknown", "unknown"),
            endpoint_with_test_status("good", "available"),
        ];

        let filtered = filter_unavailable_by_test_status(&endpoints);
        let names: Vec<&str> = filtered.iter().map(|item| item.name.as_str()).collect();

        assert_eq!(names, vec!["unknown", "good"]);
    }

    #[test]
    fn truncate_error_body_preserves_utf8_boundary() {
        let body = truncate_error_body(&"测".repeat(2000));
        assert!(body.contains("已截断"));
        assert!(body.is_char_boundary(body.len()));
    }
}

/// 非流式 Chat 响应 → 缓冲后转换为 Responses JSON 回传；记录真实 usage（codex 端点的 openai 上游路径）。
async fn buffered_responses_from_chat(
    resp: reqwest::Response,
    stats: Arc<StatsAggregator>,
    mut meta: RequestMeta,
) -> Response {
    let upstream_trace_headers = trace_capture::capture_headers(resp.headers());
    match resp.text().await {
        Ok(text) => match serde_json::from_str::<Value>(&text) {
            Ok(chat) => {
                let model = meta.model.clone().unwrap_or_default();
                let final_body =
                    serde_json::to_string_pretty(&chat_response_to_responses(&chat, &model))
                        .unwrap_or_else(|_| text.clone());
                set_upstream_trace(
                    &mut meta,
                    200,
                    upstream_trace_headers,
                    trace_capture::capture_text_body(&text),
                );
                set_response_trace(
                    &mut meta,
                    200,
                    trace_capture::json_headers(),
                    trace_capture::capture_text_body(&final_body),
                );
                let tu = usage::from_response(&chat, UpstreamFormat::OpenAiChat);
                stats.record(meta.into_record(Some(200), false, tu));
                (
                    StatusCode::OK,
                    axum::Json(chat_response_to_responses(&chat, &model)),
                )
                    .into_response()
            }
            Err(_) => json_error(StatusCode::BAD_GATEWAY, "上游响应解析失败"),
        },
        Err(e) => json_error(StatusCode::BAD_GATEWAY, &format!("读取上游响应失败: {e}")),
    }
}

/// 流式 Chat SSE → 边解析边转换为 Responses SSE 事件流回传；流结束记录真实 usage。
fn stream_responses_from_chat(
    resp: reqwest::Response,
    stats: Arc<StatsAggregator>,
    meta: RequestMeta,
) -> Response {
    let upstream_trace_headers = trace_capture::capture_headers(resp.headers());
    let (tx, rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(32);
    tokio::spawn(async move {
        let mut meta = meta;
        let mut converter =
            ResponsesStreamConverter::new(meta.model.clone().unwrap_or_default(), 0);
        let mut upstream_preview = trace_capture::TraceBodyCapture::default();
        let mut response_preview = trace_capture::TraceBodyCapture::default();
        let mut stream = resp.bytes_stream();
        let mut buf = String::new();
        let mut first = true;
        while let Some(item) = stream.next().await {
            let chunk = match item {
                Ok(c) => c,
                Err(_) => break,
            };
            if first {
                meta.first_byte_ms = Some(chrono::Utc::now().timestamp_millis() - meta.started_ms);
                first = false;
            }
            upstream_preview.push(&chunk);
            buf.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(nl) = buf.find('\n') {
                let line: String = buf[..nl].to_string();
                buf.drain(..=nl);
                let line = line.trim();
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data == "[DONE]" {
                    for ev in converter.finish() {
                        let chunk = Bytes::from(ev);
                        response_preview.push(&chunk);
                        let _ = tx.send(Ok(chunk)).await;
                    }
                } else if !data.is_empty() {
                    if let Ok(j) = serde_json::from_str::<Value>(data) {
                        for ev in converter.process_chunk(&j) {
                            let chunk = Bytes::from(ev);
                            response_preview.push(&chunk);
                            let _ = tx.send(Ok(chunk)).await;
                        }
                    }
                }
            }
        }
        // 上游未发 [DONE] 时兜底收尾（finish 幂等）
        for ev in converter.finish() {
            let chunk = Bytes::from(ev);
            response_preview.push(&chunk);
            let _ = tx.send(Ok(chunk)).await;
        }
        set_upstream_trace(
            &mut meta,
            200,
            upstream_trace_headers,
            upstream_preview.finish(),
        );
        set_response_trace(
            &mut meta,
            200,
            trace_capture::sse_headers(),
            response_preview.finish(),
        );
        let (input, output, cache_creation, cache_read) = converter.usage();
        let tu = usage::TokenUsage {
            input,
            output,
            cache_creation,
            cache_read,
        };
        stats.record(meta.into_record(Some(200), false, tu));
    });

    let body = Body::from_stream(ReceiverStream::new(rx));
    let mut response = Response::new(body);
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    response
}
