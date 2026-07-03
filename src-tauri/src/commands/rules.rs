use crate::runtime::{AppHandle, State};

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

fn normalize_rules_config(mut config: RulesConfig) -> RulesConfig {
    // 策略不再暴露给用户配置：统一固定为按端点列表顺序轮询。
    config.routing.strategy = "balanced".into();
    config.routing.model_mapping_strategy = "site-first".into();
    config
}

pub fn get_rules_config(state: State<'_, AppState>) -> AppResult<RulesConfig> {
    let conn = state.db_pool.get()?;
    Ok(normalize_rules_config(config_repo::get_config(&conn)?.rules))
}

pub async fn set_rules_config(
    app: AppHandle,
    state: State<'_, AppState>,
    config: RulesConfig,
) -> AppResult<RulesConfig> {
    let config = normalize_rules_config(config);
    save_rules_config(&state, &config)?;
    let _ = app.emit(PROXY_STATUS_EVENT, build_status(&state));
    Ok(config)
}

pub async fn reset_rules_config(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<RulesConfig> {
    let config = normalize_rules_config(RulesConfig::default());
    save_rules_config(&state, &config)?;
    let _ = app.emit(PROXY_STATUS_EVENT, build_status(&state));
    Ok(config)
}
