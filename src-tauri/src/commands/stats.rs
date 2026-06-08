use tauri::State;

use crate::error::AppResult;
use crate::models::stats::{RequestLogPage, StatsHistoryPage, StatsOverview};
use crate::modules::storage::{request_logs_repo, stats_repo};
use crate::state::AppState;

/// 四周期统计总览 + 趋势（先 flush 内存增量再聚合）。
#[tauri::command]
pub fn get_stats(state: State<AppState>) -> AppResult<StatsOverview> {
    state.stats.overview()
}

/// 请求明细分页查询（时间段[毫秒] + 可选端点过滤，按时间倒序）。
#[tauri::command]
pub fn get_request_logs(
    state: State<AppState>,
    start_ms: Option<i64>,
    end_ms: Option<i64>,
    endpoint: Option<String>,
    page: i64,
    page_size: i64,
) -> AppResult<RequestLogPage> {
    state.stats.flush()?;
    let conn = state.db_pool.get()?;
    let limit = page_size.max(1);
    let offset = (page.max(1) - 1) * limit;
    let (items, total) =
        request_logs_repo::query_page(&conn, start_ms, end_ms, endpoint.as_deref(), limit, offset)?;
    Ok(RequestLogPage { items, total })
}

/// 历史记录分页（跨全时间，按端点×日聚合行，date 倒序）。
#[tauri::command]
pub fn get_stats_history(
    state: State<AppState>,
    page: i64,
    page_size: i64,
) -> AppResult<StatsHistoryPage> {
    state.stats.flush()?;
    let conn = state.db_pool.get()?;
    let limit = page_size.max(1);
    let offset = (page.max(1) - 1) * limit;
    let (items, total) = stats_repo::history_page(&conn, limit, offset)?;
    Ok(StatsHistoryPage { items, total })
}

/// 删除单端点单日的历史记录，返回删除行数。
#[tauri::command]
pub fn delete_daily_stat(
    state: State<AppState>,
    endpoint_name: String,
    date: String,
) -> AppResult<usize> {
    let conn = state.db_pool.get()?;
    stats_repo::delete_row(&conn, &endpoint_name, &date)
}

/// 删除某一天全部端点的历史记录，返回删除行数。
#[tauri::command]
pub fn delete_stats_by_date(state: State<AppState>, date: String) -> AppResult<usize> {
    let conn = state.db_pool.get()?;
    stats_repo::delete_by_date(&conn, &date)
}
