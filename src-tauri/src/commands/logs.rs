use crate::runtime::State;

use crate::error::AppResult;
use crate::modules::logs::{self, LogLine};
use crate::modules::storage::config_repo;
use crate::state::AppState;

/// 最近日志（环形缓冲快照）。
pub fn get_recent_logs() -> AppResult<Vec<LogLine>> {
    Ok(logs::recent())
}

/// 清空日志环形缓冲（前端"清空日志"同步清后端，避免切回页面 recent() 恢复旧日志）。
pub fn clear_logs() -> AppResult<()> {
    logs::clear();
    Ok(())
}

/// 动态设置日志级别并持久化（trace/debug/info/warn/error）。
pub fn set_log_level(state: State<AppState>, level: String) -> AppResult<()> {
    logs::set_level(&level);
    let conn = state.db_pool.get()?;
    config_repo::set_value(&conn, "logLevel", &level)?;
    Ok(())
}
