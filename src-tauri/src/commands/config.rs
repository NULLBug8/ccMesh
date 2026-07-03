use std::collections::{BTreeMap, HashMap};

use crate::runtime::{AppHandle, State};

use crate::commands::proxy::{build_status, PROXY_STATUS_EVENT};
use crate::error::AppResult;
use crate::models::config::AppConfig;
use crate::modules::proxy::server::start_proxy as start_server;
use crate::modules::storage::config_repo;
use crate::state::AppState;

pub fn get_config(state: State<AppState>) -> AppResult<AppConfig> {
    let conn = state.db_pool.get()?;
    config_repo::get_config(&conn)
}

pub fn get_all_config(state: State<AppState>) -> AppResult<BTreeMap<String, String>> {
    let conn = state.db_pool.get()?;
    config_repo::get_all(&conn)
}

/// 写入若干配置键；端口变更且代理运行中则在新端口重启代理。
pub async fn set_config(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: HashMap<String, String>,
) -> AppResult<AppConfig> {
    let (needs_restart, port_changed) = {
        let conn = state.db_pool.get()?;
        let old_port = config_repo::get_value(&conn, "port")?;
        for (k, v) in &patch {
            config_repo::set_value(&conn, k, v)?;
        }
        let port_changed = patch.contains_key("port") && patch.get("port") != old_port.as_ref();
        // 代理地址 / 启用代理 / 伪装 UA 变更需重建转发 client → 重启代理使其生效
        let proxy_or_ua_changed = patch.contains_key("proxyUrl")
            || patch.contains_key("proxyEnabled")
            || patch.contains_key("openaiUa")
            || patch.contains_key("claudeCliUa");
        (port_changed || proxy_or_ua_changed, port_changed)
    };

    if needs_restart {
        let handle = state.proxy.lock().unwrap().take();
        if let Some(h) = handle {
            h.stop().await;
            let port = {
                let conn = state.db_pool.get()?;
                let port = config_repo::get_value(&conn, "port")?
                    .and_then(|v| v.parse().ok())
                    .unwrap_or_else(|| AppConfig::default().port);
                crate::models::config::port_with_env_override(port)
            };
            let new_handle =
                start_server(app.clone(), state.db_pool.clone(), port, state.stats.clone()).await?;
            *state.proxy.lock().unwrap() = Some(new_handle);
        }
    }

    // 端口变更后推送最新代理状态：运行中反映新监听端口，停机态反映新配置端口，避免仪表盘展示滞后。
    if port_changed {
        let _ = app.emit(PROXY_STATUS_EVENT, build_status(&state));
    }

    let conn = state.db_pool.get()?;
    config_repo::get_config(&conn)
}
