use rusqlite::Connection;

use crate::error::AppResult;

const MIGRATIONS: &[&str] = &[
    "CREATE TABLE IF NOT EXISTS endpoints (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        name         TEXT    NOT NULL UNIQUE,
        api_url      TEXT    NOT NULL,
        api_key      TEXT    NOT NULL DEFAULT '',
        auth_mode    TEXT    NOT NULL DEFAULT 'api_key',
        enabled      INTEGER NOT NULL DEFAULT 1,
        transformer  TEXT    NOT NULL DEFAULT 'claude',
        model        TEXT    NOT NULL DEFAULT '',
        remark       TEXT    NOT NULL DEFAULT '',
        sort_order   INTEGER NOT NULL DEFAULT 0,
        test_status  TEXT    NOT NULL DEFAULT 'unknown',
        created_at   TEXT    NOT NULL DEFAULT (datetime('now')),
        updated_at   TEXT    NOT NULL DEFAULT (datetime('now'))
    );

    CREATE TABLE IF NOT EXISTS endpoint_credentials (
        id           INTEGER PRIMARY KEY AUTOINCREMENT,
        endpoint_id  INTEGER NOT NULL,
        api_key      TEXT    NOT NULL,
        enabled      INTEGER NOT NULL DEFAULT 1,
        sort_order   INTEGER NOT NULL DEFAULT 0,
        FOREIGN KEY(endpoint_id) REFERENCES endpoints(id) ON DELETE CASCADE
    );

    CREATE TABLE IF NOT EXISTS daily_stats (
        id            INTEGER PRIMARY KEY AUTOINCREMENT,
        endpoint_name TEXT    NOT NULL,
        date          TEXT    NOT NULL,
        requests      INTEGER NOT NULL DEFAULT 0,
        errors        INTEGER NOT NULL DEFAULT 0,
        input_tokens  INTEGER NOT NULL DEFAULT 0,
        output_tokens INTEGER NOT NULL DEFAULT 0,
        device_id     TEXT    NOT NULL DEFAULT '',
        UNIQUE(endpoint_name, date, device_id)
    );
    CREATE INDEX IF NOT EXISTS idx_daily_stats_date     ON daily_stats(date);
    CREATE INDEX IF NOT EXISTS idx_daily_stats_endpoint ON daily_stats(endpoint_name);
    CREATE INDEX IF NOT EXISTS idx_daily_stats_device   ON daily_stats(device_id);

    CREATE TABLE IF NOT EXISTS credential_usage (
        id               INTEGER PRIMARY KEY AUTOINCREMENT,
        endpoint_name    TEXT    NOT NULL,
        credential_index INTEGER NOT NULL DEFAULT 0,
        date             TEXT    NOT NULL,
        requests         INTEGER NOT NULL DEFAULT 0,
        errors           INTEGER NOT NULL DEFAULT 0,
        input_tokens     INTEGER NOT NULL DEFAULT 0,
        output_tokens    INTEGER NOT NULL DEFAULT 0,
        device_id        TEXT    NOT NULL DEFAULT '',
        UNIQUE(endpoint_name, credential_index, date, device_id)
    );

    CREATE TABLE IF NOT EXISTS app_config (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );",
    "ALTER TABLE endpoints ADD COLUMN models    TEXT    NOT NULL DEFAULT '[]';
     ALTER TABLE endpoints ADD COLUMN use_proxy INTEGER NOT NULL DEFAULT 0;",
    "ALTER TABLE daily_stats ADD COLUMN cache_creation_tokens INTEGER NOT NULL DEFAULT 0;
     ALTER TABLE daily_stats ADD COLUMN cache_read_tokens     INTEGER NOT NULL DEFAULT 0;

     CREATE TABLE IF NOT EXISTS request_logs (
        id                    INTEGER PRIMARY KEY AUTOINCREMENT,
        ts                    INTEGER NOT NULL,
        endpoint_name         TEXT    NOT NULL,
        inbound_format        TEXT    NOT NULL DEFAULT '',
        upstream_url          TEXT    NOT NULL DEFAULT '',
        status_code           INTEGER,
        is_error              INTEGER NOT NULL DEFAULT 0,
        input_tokens          INTEGER NOT NULL DEFAULT 0,
        output_tokens         INTEGER NOT NULL DEFAULT 0,
        cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
        cache_read_tokens     INTEGER NOT NULL DEFAULT 0,
        model                 TEXT,
        duration_ms           INTEGER,
        device_id             TEXT    NOT NULL DEFAULT ''
     );
     CREATE INDEX IF NOT EXISTS idx_request_logs_ts       ON request_logs(ts);
     CREATE INDEX IF NOT EXISTS idx_request_logs_endpoint ON request_logs(endpoint_name);",
    "CREATE TABLE IF NOT EXISTS usage_records (
        id                    INTEGER PRIMARY KEY AUTOINCREMENT,
        app_type              TEXT    NOT NULL,
        record_key            TEXT    NOT NULL,
        date                  TEXT    NOT NULL,
        model                 TEXT    NOT NULL DEFAULT '',
        requests              INTEGER NOT NULL DEFAULT 0,
        input_tokens          INTEGER NOT NULL DEFAULT 0,
        output_tokens         INTEGER NOT NULL DEFAULT 0,
        cache_creation_tokens INTEGER NOT NULL DEFAULT 0,
        cache_read_tokens     INTEGER NOT NULL DEFAULT 0,
        UNIQUE(app_type, record_key)
     );
     CREATE INDEX IF NOT EXISTS idx_usage_records_date ON usage_records(date);
     CREATE INDEX IF NOT EXISTS idx_usage_records_app  ON usage_records(app_type);

     CREATE TABLE IF NOT EXISTS usage_sync_state (
        file_path TEXT PRIMARY KEY,
        mtime_ns  INTEGER NOT NULL
     );",
    "ALTER TABLE request_logs ADD COLUMN inbound_path  TEXT NOT NULL DEFAULT '';
     ALTER TABLE request_logs ADD COLUMN upstream_path TEXT NOT NULL DEFAULT '';",
    "ALTER TABLE request_logs ADD COLUMN first_byte_ms INTEGER;",
    "ALTER TABLE endpoints ADD COLUMN model_mappings TEXT NOT NULL DEFAULT '[]';",
    "ALTER TABLE request_logs ADD COLUMN actual_model TEXT;",
    "ALTER TABLE endpoints ADD COLUMN active_models TEXT NOT NULL DEFAULT '[]';",
    "ALTER TABLE request_logs ADD COLUMN error_body TEXT;",
    "ALTER TABLE request_logs ADD COLUMN trace_detail TEXT;",
    "ALTER TABLE endpoints ADD COLUMN balance_query TEXT NOT NULL DEFAULT '{}';",
];

pub fn run_migrations(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_version (
            version    INTEGER NOT NULL,
            applied_at TEXT    NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    let current: i64 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_version",
        [],
        |row| row.get(0),
    )?;

    for (index, script) in MIGRATIONS.iter().enumerate() {
        let version = (index + 1) as i64;
        if version > current {
            conn.execute_batch(script)?;
            conn.execute("INSERT INTO schema_version(version) VALUES (?1)", [version])?;
            tracing::info!(version, "已应用数据库迁移");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_log_columns() -> Vec<String> {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let mut stmt = conn.prepare("PRAGMA table_info(request_logs)").unwrap();
        let rows = stmt.query_map([], |row| row.get::<_, String>(1)).unwrap();
        rows.filter_map(Result::ok).collect()
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        run_migrations(&conn).unwrap();
        let version: i64 = conn
            .query_row("SELECT MAX(version) FROM schema_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, MIGRATIONS.len() as i64);

        let table_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('endpoints','daily_stats','app_config')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(table_count, 3);
    }

    #[test]
    fn v2_adds_models_and_use_proxy_columns() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let mut stmt = conn.prepare("PRAGMA table_info(endpoints)").unwrap();
        let rows = stmt.query_map([], |row| row.get::<_, String>(1)).unwrap();
        let cols: Vec<String> = rows.filter_map(Result::ok).collect();
        assert!(cols.contains(&"models".to_string()));
        assert!(cols.contains(&"use_proxy".to_string()));
    }

    #[test]
    fn v3_adds_cache_columns_and_request_logs() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let mut stmt = conn.prepare("PRAGMA table_info(daily_stats)").unwrap();
        let rows = stmt.query_map([], |row| row.get::<_, String>(1)).unwrap();
        let daily_cols: Vec<String> = rows.filter_map(Result::ok).collect();
        assert!(daily_cols.contains(&"cache_creation_tokens".to_string()));
        assert!(daily_cols.contains(&"cache_read_tokens".to_string()));

        let has_table: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='request_logs'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(has_table, 1);
    }

    #[test]
    fn v5_adds_request_log_path_columns() {
        let cols = request_log_columns();
        assert!(cols.contains(&"inbound_path".to_string()));
        assert!(cols.contains(&"upstream_path".to_string()));
    }

    #[test]
    fn v6_adds_first_byte_ms_column() {
        let cols = request_log_columns();
        assert!(cols.contains(&"first_byte_ms".to_string()));
    }

    #[test]
    fn v7_adds_model_mappings_column() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let mut stmt = conn.prepare("PRAGMA table_info(endpoints)").unwrap();
        let rows = stmt.query_map([], |row| row.get::<_, String>(1)).unwrap();
        let cols: Vec<String> = rows.filter_map(Result::ok).collect();
        assert!(cols.contains(&"model_mappings".to_string()));
    }

    #[test]
    fn v8_adds_actual_model_column() {
        let cols = request_log_columns();
        assert!(cols.contains(&"actual_model".to_string()));
    }

    #[test]
    fn v9_adds_active_models_column() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let mut stmt = conn.prepare("PRAGMA table_info(endpoints)").unwrap();
        let rows = stmt.query_map([], |row| row.get::<_, String>(1)).unwrap();
        let cols: Vec<String> = rows.filter_map(Result::ok).collect();
        assert!(cols.contains(&"active_models".to_string()));
    }

    #[test]
    fn v10_adds_error_body_column() {
        let cols = request_log_columns();
        assert!(cols.contains(&"error_body".to_string()));
    }

    #[test]
    fn v11_adds_trace_detail_column() {
        let cols = request_log_columns();
        assert!(cols.contains(&"trace_detail".to_string()));
    }

    #[test]
    fn v12_adds_balance_query_column() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        let mut stmt = conn.prepare("PRAGMA table_info(endpoints)").unwrap();
        let rows = stmt.query_map([], |row| row.get::<_, String>(1)).unwrap();
        let cols: Vec<String> = rows.filter_map(Result::ok).collect();
        assert!(cols.contains(&"balance_query".to_string()));
    }
}
