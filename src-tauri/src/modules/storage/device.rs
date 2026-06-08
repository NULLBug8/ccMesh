use rusqlite::{Connection, OptionalExtension};
use uuid::Uuid;

use crate::error::AppResult;

const DEVICE_ID_KEY: &str = "device_id";

/// 获取设备唯一 ID；不存在则生成 UUID v4 并持久化到 `app_config`。多次调用返回同一值。
pub fn get_or_create_device_id(conn: &Connection) -> AppResult<String> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT value FROM app_config WHERE key = ?1",
            [DEVICE_ID_KEY],
            |row| row.get(0),
        )
        .optional()?;

    if let Some(id) = existing {
        return Ok(id);
    }

    let id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO app_config(key, value) VALUES (?1, ?2)",
        rusqlite::params![DEVICE_ID_KEY, id],
    )?;
    Ok(id)
}
