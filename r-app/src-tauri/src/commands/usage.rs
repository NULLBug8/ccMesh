use tauri::State;

use crate::error::{AppError, AppResult};
use crate::models::usage::{DailyUsage, ModelUsage, UsageSummary, UsageSyncResult};
use crate::modules::storage::usage_repo;
use crate::modules::usage_local;
use crate::state::AppState;

/// 触发一次本机用量增量同步（读取 ~/.claude 与 ~/.codex 会话日志）。
///
/// 首次为全量解析，文件量大时耗时较长，故在阻塞线程池执行，避免卡死主线程。
#[tauri::command]
pub async fn sync_session_usage(state: State<'_, AppState>) -> AppResult<UsageSyncResult> {
    let pool = state.db_pool.clone();
    tauri::async_runtime::spawn_blocking(move || -> AppResult<UsageSyncResult> {
        let conn = pool.get()?;
        Ok(usage_local::sync_all(&conn))
    })
    .await
    .map_err(|e| AppError::Unknown(format!("用量同步任务失败: {e}")))?
}

/// 用量总览（可选日期段[YYYY-MM-DD] + app_type 过滤）。
#[tauri::command]
pub fn get_usage_summary(
    state: State<AppState>,
    start: Option<String>,
    end: Option<String>,
    app_type: Option<String>,
) -> AppResult<UsageSummary> {
    let conn = state.db_pool.get()?;
    usage_repo::summary(&conn, start.as_deref(), end.as_deref(), app_type.as_deref())
}

/// 按模型聚合用量。
#[tauri::command]
pub fn get_usage_by_model(
    state: State<AppState>,
    start: Option<String>,
    end: Option<String>,
    app_type: Option<String>,
) -> AppResult<Vec<ModelUsage>> {
    let conn = state.db_pool.get()?;
    usage_repo::by_model(&conn, start.as_deref(), end.as_deref(), app_type.as_deref())
}

/// 按天聚合用量。
#[tauri::command]
pub fn get_usage_by_day(
    state: State<AppState>,
    start: Option<String>,
    end: Option<String>,
    app_type: Option<String>,
) -> AppResult<Vec<DailyUsage>> {
    let conn = state.db_pool.get()?;
    usage_repo::by_day(&conn, start.as_deref(), end.as_deref(), app_type.as_deref())
}
