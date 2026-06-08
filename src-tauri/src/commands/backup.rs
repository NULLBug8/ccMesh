use std::fs;

use tauri::State;

use crate::error::{AppError, AppResult};
use crate::models::backup::{ConfigBundle, ImportSummary};
use crate::modules::backup;
use crate::state::AppState;

/// 导出配置迁移包到指定路径（路径由前端原生对话框选取）。
#[tauri::command]
pub fn export_config(state: State<AppState>, path: String) -> AppResult<()> {
    let conn = state.db_pool.get()?;
    let bundle = backup::build_config_bundle(&conn)?;
    let json = serde_json::to_string_pretty(&bundle)?;
    fs::write(&path, json)?;
    Ok(())
}

/// 从指定路径导入配置迁移包。strategy: "overwrite"=覆盖同名，其余=跳过同名。
#[tauri::command]
pub fn import_config(
    state: State<AppState>,
    path: String,
    strategy: String,
) -> AppResult<ImportSummary> {
    let data = fs::read_to_string(&path)?;
    let bundle: ConfigBundle = serde_json::from_str(&data)
        .map_err(|e| AppError::InvalidArgument(format!("配置文件解析失败: {e}")))?;
    let mut conn = state.db_pool.get()?;
    backup::import_config_bundle(&mut conn, &bundle, strategy == "overwrite")
}
