use std::time::Duration;

use chrono::Utc;
use serde_json::{json, Value};
use crate::runtime::State;

use crate::error::AppResult;
use crate::modules::models_cache::fetch_models_with_ua;
use crate::modules::models_probe::probe_models_with_ua;
use crate::modules::proxy::client::{build_client, should_use_proxy};
use crate::modules::storage::{config_repo, endpoint_repo};
use crate::state::AppState;

/// 模型列表（命中缓存直接返回；`force_refresh` 或过期则按启用端点拉取后缓存）。
pub async fn get_models(
    state: State<'_, AppState>,
    force_refresh: Option<bool>,
) -> AppResult<Value> {
    let force = force_refresh.unwrap_or(false);
    let ttl_min: i64 = {
        let conn = state.db_pool.get()?;
        config_repo::get_value(&conn, "modelsCacheTtl")?
            .and_then(|v| v.parse().ok())
            .unwrap_or(30)
    };

    if !force {
        let cache = state.models_cache.lock().unwrap();
        if let Some(updated) = cache.updated_at {
            if (Utc::now() - updated).num_minutes() < ttl_min {
                return Ok(json!({ "object": "list", "data": cache.models.clone() }));
            }
        }
    }

    let endpoints = {
        let conn = state.db_pool.get()?;
        endpoint_repo::list_enabled(&conn)?
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
    // 两个 client：直连 + 经代理（地址非空时）；按每端点 use_proxy||proxy_enabled 选用。
    let direct = build_client(false, "", Duration::from_secs(15))?;
    let proxied = if proxy_url.trim().is_empty() {
        None
    } else {
        Some(build_client(true, &proxy_url, Duration::from_secs(15))?)
    };

    let mut all = Vec::new();
    for ep in &endpoints {
        let want = should_use_proxy(ep.use_proxy, proxy_enabled, &proxy_url);
        let client = if want {
            proxied.as_ref().unwrap_or(&direct)
        } else {
            &direct
        };
        all.extend(fetch_models_with_ua(client, ep, Some(&openai_ua), Some(&claude_cli_ua)).await);
    }

    {
        let mut cache = state.models_cache.lock().unwrap();
        cache.models = all.clone();
        cache.updated_at = Some(Utc::now());
    }
    Ok(json!({ "object": "list", "data": all }))
}

/// 拉取指定上游端点的可用模型 id 列表（供端点表单「刷新」按钮；端点可能尚未保存，故按字段传参）。
/// 走鉴权聚合 + URL 候选探测策略，提高未知协议端点的成功率。
/// 代理决策：表单的 use_proxy 或全局 proxyEnabled（且地址非空）。
pub async fn fetch_endpoint_models(
    state: State<'_, AppState>,
    api_url: String,
    api_key: String,
    transformer: String,
    use_proxy: Option<bool>,
) -> AppResult<Vec<String>> {
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
    let want = should_use_proxy(use_proxy.unwrap_or(false), proxy_enabled, &proxy_url);
    let client = build_client(want, &proxy_url, Duration::from_secs(15))?;
    Ok(probe_models_with_ua(
        &client,
        &api_url,
        &api_key,
        &transformer,
        Some(&openai_ua),
        Some(&claude_cli_ua),
    )
    .await)
}
