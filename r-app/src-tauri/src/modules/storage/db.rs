use std::path::Path;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

use crate::error::{AppError, AppResult};

/// 全局连接池类型别名。
pub type DbPool = Pool<SqliteConnectionManager>;

/// 创建 SQLite 连接池：启用 WAL、busy_timeout、外键。
pub fn create_pool(db_file: &Path) -> AppResult<DbPool> {
    let manager = SqliteConnectionManager::file(db_file).with_init(|c| {
        c.execute_batch(
            "PRAGMA journal_mode=WAL;\
             PRAGMA busy_timeout=5000;\
             PRAGMA foreign_keys=ON;\
             PRAGMA synchronous=NORMAL;",
        )
    });
    Pool::builder()
        .build(manager)
        .map_err(|e| AppError::Db(format!("创建连接池失败: {e}")))
}
