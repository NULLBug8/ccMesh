use rusqlite::OptionalExtension;
use tauri::{AppHandle, Emitter, State};

use crate::error::{AppError, AppResult};
use crate::models::proxy::ProxyStatus;
use crate::modules::proxy::server::start_proxy as start_server;
use crate::modules::storage::endpoint_repo;
use crate::state::AppState;

const DEFAULT_PORT: u16 = 3000;
const PROXY_STATUS_EVENT: &str = "proxy-status-changed";

fn read_port(state: &AppState) -> u16 {
    if let Ok(conn) = state.db_pool.get() {
        if let Ok(Some(v)) = conn
            .query_row(
                "SELECT value FROM app_config WHERE key = 'proxy_port'",
                [],
                |r| r.get::<_, String>(0),
            )
            .optional()
        {
            if let Ok(p) = v.parse::<u16>() {
                return p;
            }
        }
    }
    DEFAULT_PORT
}

fn enabled_count(state: &AppState) -> usize {
    state
        .db_pool
        .get()
        .ok()
        .and_then(|c| endpoint_repo::list_enabled(&c).ok())
        .map(|v| v.len())
        .unwrap_or(0)
}

fn build_status(state: &AppState) -> ProxyStatus {
    let guard = state.proxy.lock().unwrap();
    match guard.as_ref() {
        Some(h) => ProxyStatus {
            running: true,
            port: h.port,
            current_endpoint: h.current_endpoint(),
            enabled_endpoint_count: enabled_count(state),
        },
        None => ProxyStatus {
            running: false,
            port: read_port(state),
            current_endpoint: None,
            enabled_endpoint_count: enabled_count(state),
        },
    }
}

fn emit_status(app: &AppHandle, status: &ProxyStatus) {
    let _ = app.emit(PROXY_STATUS_EVENT, status);
}

#[tauri::command]
pub async fn start_proxy(app: AppHandle, state: State<'_, AppState>) -> AppResult<ProxyStatus> {
    {
        // 已运行则直接返回当前状态（不重复绑定端口）
        if state.proxy.lock().unwrap().is_some() {
            return Ok(build_status(&state));
        }
    }
    let port = read_port(&state);
    let pool = state.db_pool.clone();
    let handle = start_server(pool, port, state.stats.clone()).await?;
    {
        *state.proxy.lock().unwrap() = Some(handle);
    }
    let status = build_status(&state);
    emit_status(&app, &status);
    Ok(status)
}

#[tauri::command]
pub async fn stop_proxy(app: AppHandle, state: State<'_, AppState>) -> AppResult<ProxyStatus> {
    let handle = { state.proxy.lock().unwrap().take() };
    if let Some(h) = handle {
        h.stop().await;
    }
    let status = build_status(&state);
    emit_status(&app, &status);
    Ok(status)
}

#[tauri::command]
pub fn get_proxy_status(state: State<'_, AppState>) -> AppResult<ProxyStatus> {
    Ok(build_status(&state))
}

#[tauri::command]
pub fn switch_endpoint(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
) -> AppResult<ProxyStatus> {
    {
        let guard = state.proxy.lock().unwrap();
        let h = guard
            .as_ref()
            .ok_or_else(|| AppError::Proxy("代理未运行".to_string()))?;
        h.switch_endpoint(&name)?;
    }
    let status = build_status(&state);
    emit_status(&app, &status);
    Ok(status)
}
