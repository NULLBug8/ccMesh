use serde::Serialize;
use serde_json::json;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::UpdaterExt;

use crate::error::{AppError, AppResult};
use crate::modules::proxy::client::should_proxy_update;
use crate::modules::storage::config_repo;
use crate::state::AppState;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub available: bool,
    pub version: String,
    pub current_version: String,
    pub notes: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettings {
    pub auto_check: bool,
    pub check_interval: i64,
    pub skipped_version: String,
}

/// 构建 updater：`proxyForUpdate` 且地址非空时经代理出网（无 scheme 按 http 处理；无效则告警直连）。
fn build_updater(
    app: &AppHandle,
    state: &State<'_, AppState>,
) -> AppResult<tauri_plugin_updater::Updater> {
    let cfg = {
        let conn = state.db_pool.get()?;
        config_repo::get_config(&conn)?
    };
    let mut builder = app.updater_builder();
    if should_proxy_update(cfg.proxy_for_update, &cfg.proxy_url) {
        let raw = cfg.proxy_url.trim();
        let normalized = if raw.contains("://") {
            raw.to_string()
        } else {
            format!("http://{raw}")
        };
        match normalized.parse() {
            Ok(u) => builder = builder.proxy(u),
            Err(e) => tracing::warn!("更新代理地址无效，直连检查更新: {e}"),
        }
    }
    builder
        .build()
        .map_err(|e| AppError::Unknown(format!("更新器不可用: {e}")))
}

/// 检查更新（endpoints/pubkey 未配置时返回错误，由前端容错处理）。
#[tauri::command]
pub async fn check_for_updates(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<UpdateInfo> {
    let current = app.package_info().version.to_string();
    let updater = build_updater(&app, &state)?;
    match updater.check().await {
        Ok(Some(u)) => Ok(UpdateInfo {
            available: true,
            version: u.version.clone(),
            current_version: u.current_version.clone(),
            notes: u.body.clone().unwrap_or_default(),
        }),
        Ok(None) => Ok(UpdateInfo {
            available: false,
            version: String::new(),
            current_version: current,
            notes: String::new(),
        }),
        Err(e) => Err(AppError::Unknown(format!("检查更新失败: {e}"))),
    }
}

/// 下载并安装更新；通过 `update-progress` 事件推送进度。
#[tauri::command]
pub async fn download_and_install(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<()> {
    let updater = build_updater(&app, &state)?;
    let update = updater
        .check()
        .await
        .map_err(|e| AppError::Unknown(format!("检查更新失败: {e}")))?
        .ok_or_else(|| AppError::Unknown("无可用更新".to_string()))?;

    let mut downloaded: u64 = 0;
    let app_progress = app.clone();
    update
        .download_and_install(
            move |chunk, total| {
                downloaded += chunk as u64;
                let _ = app_progress.emit(
                    "update-progress",
                    json!({ "downloaded": downloaded, "total": total }),
                );
            },
            || {},
        )
        .await
        .map_err(|e| AppError::Unknown(format!("下载安装失败: {e}")))?;
    Ok(())
}

#[tauri::command]
pub fn get_update_settings(state: State<AppState>) -> AppResult<UpdateSettings> {
    let conn = state.db_pool.get()?;
    let cfg = config_repo::get_config(&conn)?;
    Ok(UpdateSettings {
        auto_check: cfg.update.auto_check,
        check_interval: cfg.update.check_interval,
        skipped_version: cfg.update.skipped_version,
    })
}

#[tauri::command]
pub fn set_update_settings(
    state: State<AppState>,
    auto_check: bool,
    check_interval: i64,
) -> AppResult<()> {
    let conn = state.db_pool.get()?;
    config_repo::set_value(&conn, "update_autoCheck", &auto_check.to_string())?;
    config_repo::set_value(&conn, "update_checkInterval", &check_interval.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn skip_version(state: State<AppState>, version: String) -> AppResult<()> {
    let conn = state.db_pool.get()?;
    config_repo::set_value(&conn, "update_skippedVersion", &version)?;
    Ok(())
}
