use std::future;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::{DefaultBodyLimit, State};
use axum::http::HeaderMap;
use axum::middleware::from_fn;
use axum::routing::{get, post};
use axum::{
    body::Bytes,
    response::{IntoResponse, Response},
    Json, Router,
};
use serde_json::json;
use crate::runtime::AppHandle;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::error::{AppError, AppResult};
use crate::modules::models_cache::model_info;
use crate::modules::proxy::circuit_breaker::{BreakerRegistry, CircuitBreakerConfig};
use crate::modules::proxy::forward::{handle_proxy, ActiveRequests, ProxyState};
use crate::modules::proxy::rotation::Rotation;
use crate::modules::stats::aggregator::StatsAggregator;
use crate::modules::storage::{config_repo, db::DbPool, endpoint_repo};
use crate::modules::transform::thinking_rectifier::RectifierConfig;

const PROXY_REQUEST_BODY_LIMIT_BYTES: usize = 64 * 1024 * 1024;

#[cfg(test)]
fn web_admin_root_asset_path(path: &str) -> Option<String> {
    match path {
        "/" => Some(String::new()),
        "/favicon.ico" => Some("favicon.ico".to_string()),
        _ => path
            .strip_prefix("/assets/")
            .map(|asset_path| format!("assets/{asset_path}")),
    }
}

/// 代理运行句柄，存于 `AppState.proxy`。持有关停信号、任务句柄与共享状态。
pub struct ProxyHandle {
    pub port: u16,
    shutdown: Option<oneshot::Sender<()>>,
    join: Option<JoinHandle<()>>,
    pub state: Arc<ProxyState>,
}

impl ProxyHandle {
    /// 优雅停止代理服务并释放端口。
    pub async fn stop(mut self) {
        if let Some(tx) = self.shutdown.take() {
            let _ = tx.send(());
        }
        if let Some(j) = self.join.take() {
            let _ = j.await;
        }
    }

    pub fn current_endpoint(&self) -> Option<String> {
        self.state.current_endpoint_name()
    }

    /// 手动切换到指定端点名：定位索引、取消旧端点在途请求、置为当前。
    pub fn switch_endpoint(&self, name: &str) -> AppResult<String> {
        let conn = self.state.db_pool.get()?;
        let enabled = endpoint_repo::list_enabled(&conn)?;
        let idx = enabled
            .iter()
            .position(|e| e.name.eq_ignore_ascii_case(name.trim()))
            .ok_or_else(|| AppError::NotFound(format!("端点 '{name}' 不存在或未启用")))?;

        if let Some(old) = self.state.current_endpoint_name() {
            self.state.active.cancel(&old);
        }
        self.state.rotation.set_index(idx);
        let new_name = enabled[idx].name.clone();
        *self.state.current_endpoint.lock().unwrap() = Some(new_name.clone());
        Ok(new_name)
    }
}

fn build_router(state: Arc<ProxyState>) -> Router {
    let protected = Router::new()
        .route("/", get(crate::commands::web_admin::static_asset_root))
        .route(
            "/assets/{*path}",
            get(crate::commands::web_admin::static_asset_root_assets),
        )
        .route(
            "/favicon.ico",
            get(crate::commands::web_admin::static_asset_favicon),
        )
        .route(
            "/__admin/api/invoke",
            post(crate::commands::web_admin::invoke_http),
        )
        .route(
            "/__admin/events",
            get(crate::commands::web_admin::events_sse),
        )
        .route(
            "/__admin",
            get(crate::commands::web_admin::static_asset_root),
        )
        .route(
            "/__admin/",
            get(crate::commands::web_admin::static_asset_root),
        )
        .route(
            "/__admin/{*path}",
            get(crate::commands::web_admin::static_asset),
        )
        .route("/health", get(health_route))
        .route("/stats", get(stats_route))
        .route("/v1/models", get(models_route))
        .route("/v1/messages/count_tokens", post(count_tokens_route))
        .fallback(handle_proxy)
        .layer(from_fn(crate::modules::auth::require_admin_auth));

    Router::new()
        .route(
            "/__auth/login",
            get(crate::modules::auth::login_page).post(crate::modules::auth::login),
        )
        .route(
            "/__auth/logout",
            get(crate::modules::auth::logout).post(crate::modules::auth::logout),
        )
        .merge(protected)
        .layer(DefaultBodyLimit::max(PROXY_REQUEST_BODY_LIMIT_BYTES))
        .with_state(state)
}

/// 在本地端口启动代理服务。返回运行句柄。
pub async fn start_proxy(
    app: AppHandle,
    db_pool: DbPool,
    port: u16,
    stats: Arc<StatsAggregator>,
) -> AppResult<ProxyHandle> {
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(90))
        .timeout(Duration::from_secs(300))
        .connect_timeout(Duration::from_secs(30))
        .no_proxy()
        .build()
        .map_err(|e| AppError::Proxy(format!("构建 HTTP 客户端失败: {e}")))?;

    // 读取代理地址与伪装 UA 配置
    let cfg = {
        let conn = db_pool.get()?;
        config_repo::get_config(&conn)?
    };
    let proxy_client = if cfg.proxy_url.trim().is_empty() {
        None
    } else {
        match reqwest::Proxy::all(cfg.proxy_url.trim()) {
            Ok(proxy) => reqwest::Client::builder()
                .pool_max_idle_per_host(10)
                .pool_idle_timeout(Duration::from_secs(90))
                .timeout(Duration::from_secs(300))
                .connect_timeout(Duration::from_secs(30))
                .proxy(proxy)
                .build()
                .ok(),
            Err(e) => {
                tracing::warn!("代理地址无效，回落直连: {e}");
                None
            }
        }
    };

    let state = Arc::new(ProxyState {
        app,
        db_pool,
        client,
        proxy_client,
        openai_ua: std::sync::Mutex::new(cfg.openai_ua),
        claude_cli_ua: std::sync::Mutex::new(cfg.claude_cli_ua),
        rotation: Rotation::new(),
        active: ActiveRequests::default(),
        stats,
        current_endpoint: Mutex::new(None),
        proxy_enabled: cfg.proxy_enabled,
        breakers: BreakerRegistry::new(CircuitBreakerConfig::from_rules(
            &cfg.rules.circuit_breaker,
        )),
        rectifier_config: RectifierConfig::from_rules(&cfg.rules.degradation),
        reasoning_effort_fallback: cfg.rules.degradation.reasoning_effort_fallback,
        model_mapping_strategy: cfg.rules.routing.model_mapping_strategy,
        max_retries: cfg.rules.routing.max_retries,
    });

    let app = build_router(state.clone());

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| AppError::Proxy(format!("绑定端口 {port} 失败: {e}")))?;
    let actual_port = listener.local_addr().map(|a| a.port()).unwrap_or(port);

    let (tx, rx) = oneshot::channel::<()>();
    let join = tokio::spawn(async move {
        let server = axum::serve(listener, app).with_graceful_shutdown(async move {
            match rx.await {
                Ok(()) => {}
                Err(_) => {
                    tracing::error!(
                        "代理服务关闭信号发送端被丢弃，保持监听以避免 Web 管理端 Failed to fetch"
                    );
                    future::pending::<()>().await;
                }
            }
        });
        if let Err(e) = server.await {
            tracing::error!("代理服务退出: {e}");
        }
    });

    tracing::info!(port = actual_port, "代理服务已启动");
    Ok(ProxyHandle {
        port: actual_port,
        shutdown: Some(tx),
        join: Some(join),
        state,
    })
}

async fn health_route() -> Response {
    Json(json!({ "status": "healthy" })).into_response()
}

async fn stats_route() -> Response {
    Json(json!({})).into_response()
}

#[cfg(test)]
mod tests {
    use super::{web_admin_root_asset_path, PROXY_REQUEST_BODY_LIMIT_BYTES};

    #[test]
    fn root_path_serves_web_admin_shell() {
        assert_eq!(web_admin_root_asset_path("/"), Some(String::new()));
    }

    #[test]
    fn vite_assets_are_served_without_stealing_api_routes() {
        assert_eq!(
            web_admin_root_asset_path("/assets/index.js"),
            Some("assets/index.js".to_string())
        );
        assert_eq!(web_admin_root_asset_path("/v1/messages"), None);
        assert_eq!(web_admin_root_asset_path("/v1/chat/completions"), None);
    }

    #[test]
    fn proxy_body_limit_allows_large_tool_payloads() {
        assert!(PROXY_REQUEST_BODY_LIMIT_BYTES >= 64 * 1024 * 1024);
    }
}

/// `/v1/models`：按启用端点的配置态模型清单聚合（读库，不请求上游）。
/// 专用型端点（model 非空）公布锁定模型；聚合型端点展开 models 清单。
/// 按入站鉴权格式返回：带 x-api-key/anthropic-version → Anthropic 格式；否则 OpenAI 格式。
async fn models_route(State(st): State<Arc<ProxyState>>, headers: HeaderMap) -> Response {
    let pairs: Vec<(String, String)> = match st.db_pool.get() {
        Ok(conn) => match endpoint_repo::list_enabled(&conn) {
            Ok(endpoints) => endpoints
                .iter()
                .flat_map(|ep| {
                    crate::modules::proxy::resolver::advertised_models(ep)
                        .into_iter()
                        .map(|m| (m, ep.name.clone()))
                        .collect::<Vec<_>>()
                })
                .collect(),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    };

    // 跨端点去重（大小写不敏感，保留首次出现），与对外公布模型口径一致，
    // 避免多个端点公布同名模型时 /v1/models 出现重复项（拉取模型下拉重复）。
    let pairs = crate::modules::proxy::resolver::dedup_advertised_pairs(pairs);

    let anthropic = headers.contains_key("x-api-key") || headers.contains_key("anthropic-version");
    if anthropic {
        // Anthropic 格式：data[].{id,type,display_name,created_at} + first_id/last_id/has_more
        let data: Vec<serde_json::Value> = pairs
            .iter()
            .map(|(id, _)| {
                json!({
                    "id": id,
                    "type": "model",
                    "display_name": id,
                    "created_at": "2025-01-01T00:00:00Z"
                })
            })
            .collect();
        // 空列表时 first_id/last_id 为 null（对齐官方 Anthropic 行为）
        let first_id = pairs.first().map(|(id, _)| id.as_str());
        let last_id = pairs.last().map(|(id, _)| id.as_str());
        Json(json!({
            "data": data,
            "first_id": first_id,
            "last_id": last_id,
            "has_more": false
        }))
        .into_response()
    } else {
        // OpenAI 格式：object:list + data[].{id,object,created,owned_by}
        let data: Vec<serde_json::Value> = pairs
            .iter()
            .map(|(id, name)| model_info(id, name))
            .collect();
        Json(json!({ "object": "list", "data": data })).into_response()
    }
}

/// `/v1/messages/count_tokens`：解析请求体 system/messages，返回输入 token 估算。
async fn count_tokens_route(body: Bytes) -> Response {
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    let system = json.get("system");
    let messages = json.get("messages").cloned().unwrap_or_else(|| json!([]));
    let input = crate::modules::tokens::estimate_input_tokens(system, &messages);
    Json(json!({ "input_tokens": input })).into_response()
}
