use tauri::{AppHandle, Emitter, State};

use crate::commands::proxy::{build_status, PROXY_STATUS_EVENT};
use crate::error::AppResult;
use crate::models::rules::RulesConfig;
use crate::modules::proxy::server::start_proxy as start_server;
use crate::modules::storage::config_repo;
use crate::state::AppState;

fn save_rules_config(state: &AppState, config: &RulesConfig) -> AppResult<()> {
    let conn = state.db_pool.get()?;
    config_repo::set_value(&conn, "rulesConfig", &serde_json::to_string(config)?)?;
    Ok(())
}

async fn restart_proxy_if_running(app: &AppHandle, state: &AppState) -> AppResult<()> {
    let handle = state.proxy.lock().unwrap().take();
    if let Some(handle) = handle {
        handle.stop().await;
        let port = {
            let conn = state.db_pool.get()?;
            config_repo::get_config(&conn)?.port
        };
        let new_handle = start_server(
            app.clone(),
            state.db_pool.clone(),
            port,
            state.stats.clone(),
        )
        .await?;
        *state.proxy.lock().unwrap() = Some(new_handle);
    }
    Ok(())
}

#[tauri::command]
pub fn get_rules_config(state: State<'_, AppState>) -> AppResult<RulesConfig> {
    let conn = state.db_pool.get()?;
    Ok(config_repo::get_config(&conn)?.rules)
}

#[tauri::command]
pub async fn set_rules_config(
    app: AppHandle,
    state: State<'_, AppState>,
    config: RulesConfig,
) -> AppResult<RulesConfig> {
    save_rules_config(&state, &config)?;
    restart_proxy_if_running(&app, &state).await?;
    let _ = app.emit(PROXY_STATUS_EVENT, build_status(&state));
    Ok(config)
}

#[tauri::command]
pub async fn reset_rules_config(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<RulesConfig> {
    let config = RulesConfig::default();
    save_rules_config(&state, &config)?;
    restart_proxy_if_running(&app, &state).await?;
    let _ = app.emit(PROXY_STATUS_EVENT, build_status(&state));
    Ok(config)
}
