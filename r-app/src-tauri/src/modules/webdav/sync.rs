use std::path::Path;

use rusqlite::{params, params_from_iter, Connection};

use crate::error::AppResult;
use crate::modules::storage::config_repo::SAFE_CONFIG_KEYS;

fn sql_quote(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "''")
}

fn placeholders(n: usize) -> String {
    vec!["?"; n].join(",")
}

/// 生成可上传的数据库副本：VACUUM INTO 临时文件，并剔除设备特定配置（仅保留安全键）。
pub fn create_backup_copy(conn: &Connection, temp_path: &Path) -> AppResult<()> {
    if temp_path.exists() {
        let _ = std::fs::remove_file(temp_path);
    }
    conn.execute_batch(&format!("VACUUM INTO '{}'", sql_quote(temp_path)))?;

    let backup = Connection::open(temp_path)?;
    backup.execute(
        &format!(
            "DELETE FROM app_config WHERE key NOT IN ({})",
            placeholders(SAFE_CONFIG_KEYS.len())
        ),
        params_from_iter(SAFE_CONFIG_KEYS.iter()),
    )?;
    Ok(())
}

/// 将备份库 ATTACH 后合并到本地：
/// - app_config：仅安全键；overwrite=REPLACE / keep=IGNORE
/// - endpoints：按 name；overwrite=REPLACE / keep=IGNORE
/// - daily_stats：重打本地 device_id 并按 (endpoint,date) 聚合；overwrite 先删冲突日
pub fn merge_from_backup(
    conn: &mut Connection,
    backup_path: &Path,
    overwrite: bool,
    device_id: &str,
) -> AppResult<()> {
    conn.execute_batch(&format!(
        "ATTACH DATABASE '{}' AS backup",
        sql_quote(backup_path)
    ))?;

    let result = (|| -> AppResult<()> {
        let mode = if overwrite { "OR REPLACE" } else { "OR IGNORE" };
        let tx = conn.transaction()?;

        tx.execute(
            &format!(
                "INSERT {mode} INTO app_config(key, value)
                 SELECT key, value FROM backup.app_config WHERE key IN ({})",
                placeholders(SAFE_CONFIG_KEYS.len())
            ),
            params_from_iter(SAFE_CONFIG_KEYS.iter()),
        )?;

        tx.execute(
            &format!(
                "INSERT {mode} INTO endpoints
                    (name, api_url, api_key, auth_mode, enabled, transformer, model, remark, sort_order, test_status)
                 SELECT name, api_url, api_key, auth_mode, enabled, transformer, model, remark, sort_order, test_status
                 FROM backup.endpoints"
            ),
            [],
        )?;

        if overwrite {
            tx.execute(
                "DELETE FROM daily_stats WHERE EXISTS (
                    SELECT 1 FROM backup.daily_stats b
                    WHERE b.endpoint_name = daily_stats.endpoint_name AND b.date = daily_stats.date
                 )",
                [],
            )?;
        }
        tx.execute(
            &format!(
                "INSERT {mode} INTO daily_stats
                    (endpoint_name, date, requests, errors, input_tokens, output_tokens, device_id)
                 SELECT endpoint_name, date, SUM(requests), SUM(errors), SUM(input_tokens), SUM(output_tokens), ?1
                 FROM backup.daily_stats GROUP BY endpoint_name, date"
            ),
            params![device_id],
        )?;

        tx.commit()?;
        Ok(())
    })();

    let _ = conn.execute_batch("DETACH DATABASE backup");
    result
}
