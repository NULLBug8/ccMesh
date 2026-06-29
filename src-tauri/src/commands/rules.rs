use tauri::{AppHandle, Emitter, State};

use crate::commands::proxy::{build_status, PROXY_STATUS_EVENT};
use crate::error::AppResult;
use crate::models::rules::RulesConfig;
use crate::modules::storage::config_repo;
use crate::state::AppState;

fn save_rules_config(state: &AppState, config: &RulesConfig) -> AppResult<()> {
    let conn = state.db_pool.get()?;
    config_repo::set_value(&conn, "rulesConfig", &serde_json::to_string(config)?)?;
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
    let _ = app.emit(PROXY_STATUS_EVENT, build_status(&state));
    Ok(config)
}
