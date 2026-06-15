//! 配置文件管理命令（Claude Code / Codex 渠道的 抽取/存储/应用/覆盖）。

use serde_json::Value;
use tauri::AppHandle;

use crate::error::AppResult;
use crate::models::tool_config::{
    ChannelData, ChannelMeta, ClaudeOperationFields, CodexOperationFields, ExtractResult,
    SaveChannelRequest,
};
use crate::modules::tool_config::{self as tc, Tool};

#[tauri::command]
pub fn list_profile_channels(app: AppHandle, app_type: String) -> AppResult<Vec<ChannelMeta>> {
    tc::list_channels(&app, Tool::from_str(&app_type)?)
}

#[tauri::command]
pub fn get_profile_channel(
    app: AppHandle,
    app_type: String,
    id: String,
) -> AppResult<ChannelData> {
    tc::get_channel(&app, Tool::from_str(&app_type)?, &id)
}

#[tauri::command]
pub fn save_profile_channel(
    app: AppHandle,
    app_type: String,
    req: SaveChannelRequest,
) -> AppResult<ChannelMeta> {
    tc::save_channel(&app, Tool::from_str(&app_type)?, req)
}

#[tauri::command]
pub fn delete_profile_channel(app: AppHandle, app_type: String, id: String) -> AppResult<()> {
    tc::delete_channel(&app, Tool::from_str(&app_type)?, &id)
}

#[tauri::command]
pub fn extract_source_record(app: AppHandle, app_type: String) -> AppResult<ExtractResult> {
    tc::extract_record(&app, Tool::from_str(&app_type)?)
}

#[tauri::command]
pub fn apply_profile_config(
    app: AppHandle,
    app_type: String,
    snapshot: Value,
) -> AppResult<()> {
    tc::apply_config(&app, Tool::from_str(&app_type)?, snapshot)
}

#[tauri::command]
pub fn preview_claude_settings(base: Value, fields: ClaudeOperationFields) -> AppResult<Value> {
    Ok(tc::claude::merge_operation_fields(&base, &fields))
}

#[tauri::command]
pub fn parse_claude_fields(snapshot: Value) -> AppResult<ClaudeOperationFields> {
    Ok(tc::claude::parse_operation_fields(&snapshot))
}

#[tauri::command]
pub fn preview_codex_config(
    config_toml: String,
    fields: CodexOperationFields,
    goal_mode: Option<bool>,
) -> AppResult<String> {
    tc::codex::build_codex_config(&config_toml, &fields, goal_mode)
}

#[tauri::command]
pub fn parse_codex_fields(
    auth: Value,
    config_toml: String,
) -> AppResult<CodexOperationFields> {
    Ok(tc::codex::parse_operation_fields(&auth, &config_toml))
}
