use tauri::State;

use crate::error::AppResult;
use crate::models::stats::{DailyStat, StatsOverview};
use crate::modules::storage::stats_repo;
use crate::state::AppState;

/// 四周期统计总览 + 趋势（先 flush 内存增量再聚合）。
#[tauri::command]
pub fn get_stats(state: State<AppState>) -> AppResult<StatsOverview> {
    state.stats.overview()
}

/// 有数据的归档月份列表（"YYYY-MM" 倒序）。
#[tauri::command]
pub fn get_archive_months(state: State<AppState>) -> AppResult<Vec<String>> {
    state.stats.flush()?;
    let conn = state.db_pool.get()?;
    stats_repo::archive_months(&conn)
}

/// 某月每端点每日明细。
#[tauri::command]
pub fn get_monthly_archive(state: State<AppState>, month: String) -> AppResult<Vec<DailyStat>> {
    state.stats.flush()?;
    let conn = state.db_pool.get()?;
    stats_repo::monthly_data(&conn, &month)
}

/// 删除某月统计，返回删除行数。
#[tauri::command]
pub fn delete_monthly_stats(state: State<AppState>, month: String) -> AppResult<usize> {
    let conn = state.db_pool.get()?;
    stats_repo::delete_month(&conn, &month)
}
