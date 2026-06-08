use rusqlite::{params, params_from_iter, types::Value as SqlValue, Connection};

use crate::error::AppResult;
use crate::models::stats::RequestLog;

/// 批量插入请求明细（同一设备）。空切片为空操作。
pub fn insert_batch(conn: &mut Connection, logs: &[RequestLog], device_id: &str) -> AppResult<()> {
    if logs.is_empty() {
        return Ok(());
    }
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO request_logs(
                ts, endpoint_name, inbound_format, upstream_url, inbound_path, upstream_path,
                status_code, is_error, input_tokens, output_tokens, cache_creation_tokens,
                cache_read_tokens, model, duration_ms, device_id)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        )?;
        for l in logs {
            stmt.execute(params![
                l.ts,
                l.endpoint_name,
                l.inbound_format,
                l.upstream_url,
                l.inbound_path,
                l.upstream_path,
                l.status_code,
                l.is_error as i64,
                l.input_tokens,
                l.output_tokens,
                l.cache_creation_tokens,
                l.cache_read_tokens,
                l.model,
                l.duration_ms,
                device_id,
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn row_to_log(r: &rusqlite::Row) -> rusqlite::Result<RequestLog> {
    Ok(RequestLog {
        id: r.get(0)?,
        ts: r.get(1)?,
        endpoint_name: r.get(2)?,
        inbound_format: r.get(3)?,
        upstream_url: r.get(4)?,
        status_code: r.get(5)?,
        is_error: r.get::<_, i64>(6)? != 0,
        input_tokens: r.get(7)?,
        output_tokens: r.get(8)?,
        cache_creation_tokens: r.get(9)?,
        cache_read_tokens: r.get(10)?,
        model: r.get(11)?,
        duration_ms: r.get(12)?,
        inbound_path: r.get(13)?,
        upstream_path: r.get(14)?,
    })
}

/// 分页查询请求明细（按 ts 倒序）。可选时间段（毫秒，闭区间）与端点过滤。
/// 返回 (当前页明细, 满足过滤的总条数)。
pub fn query_page(
    conn: &Connection,
    start_ms: Option<i64>,
    end_ms: Option<i64>,
    endpoint: Option<&str>,
    limit: i64,
    offset: i64,
) -> AppResult<(Vec<RequestLog>, i64)> {
    let mut where_sql = String::from(" WHERE 1=1");
    let mut args: Vec<SqlValue> = Vec::new();
    if let Some(s) = start_ms {
        where_sql.push_str(" AND ts >= ?");
        args.push(SqlValue::Integer(s));
    }
    if let Some(e) = end_ms {
        where_sql.push_str(" AND ts <= ?");
        args.push(SqlValue::Integer(e));
    }
    if let Some(ep) = endpoint {
        if !ep.is_empty() {
            where_sql.push_str(" AND endpoint_name = ?");
            args.push(SqlValue::Text(ep.to_string()));
        }
    }

    let total: i64 = {
        let sql = format!("SELECT COUNT(*) FROM request_logs{where_sql}");
        conn.query_row(&sql, params_from_iter(args.iter()), |r| r.get(0))?
    };

    let mut page_args = args.clone();
    page_args.push(SqlValue::Integer(limit));
    page_args.push(SqlValue::Integer(offset));
    let sql = format!(
        "SELECT id, ts, endpoint_name, inbound_format, upstream_url, status_code, is_error,
                input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens, model, duration_ms,
                inbound_path, upstream_path
         FROM request_logs{where_sql}
         ORDER BY ts DESC, id DESC LIMIT ? OFFSET ?"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(page_args.iter()), row_to_log)?;
    let mut items = Vec::new();
    for r in rows {
        items.push(r?);
    }
    Ok((items, total))
}

/// 删除 `cutoff_ms` 之前的明细行，返回删除行数。
pub fn prune_older_than(conn: &Connection, cutoff_ms: i64) -> AppResult<usize> {
    let n = conn.execute("DELETE FROM request_logs WHERE ts < ?1", params![cutoff_ms])?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::storage::migration::run_migrations;

    fn db() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        run_migrations(&c).unwrap();
        c
    }

    fn log(ts: i64, endpoint: &str, is_error: bool) -> RequestLog {
        RequestLog {
            id: 0,
            ts,
            endpoint_name: endpoint.to_string(),
            inbound_format: "claude".to_string(),
            upstream_url: "https://x".to_string(),
            inbound_path: "/v1/messages".to_string(),
            upstream_path: "/v1/chat/completions".to_string(),
            status_code: Some(200),
            is_error,
            input_tokens: 10,
            output_tokens: 5,
            cache_creation_tokens: 1,
            cache_read_tokens: 2,
            model: Some("m".to_string()),
            duration_ms: Some(123),
        }
    }

    #[test]
    fn insert_and_query_paginates_desc() {
        let mut c = db();
        insert_batch(&mut c, &[log(100, "a", false), log(200, "b", true), log(300, "a", false)], "dev").unwrap();
        let (page1, total) = query_page(&c, None, None, None, 2, 0).unwrap();
        assert_eq!(total, 3);
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].ts, 300); // 倒序
        assert_eq!(page1[1].ts, 200);
        let (page2, _) = query_page(&c, None, None, None, 2, 2).unwrap();
        assert_eq!(page2.len(), 1);
        assert_eq!(page2[0].ts, 100);
        assert!(page1[1].is_error);
        assert_eq!(page1[0].cache_read_tokens, 2);
    }

    #[test]
    fn query_filters_by_time_and_endpoint() {
        let mut c = db();
        insert_batch(&mut c, &[log(100, "a", false), log(200, "b", false), log(300, "a", false)], "dev").unwrap();
        let (items, total) = query_page(&c, Some(150), Some(350), None, 50, 0).unwrap();
        assert_eq!(total, 2);
        assert_eq!(items.len(), 2);
        let (a_items, a_total) = query_page(&c, None, None, Some("a"), 50, 0).unwrap();
        assert_eq!(a_total, 2);
        assert!(a_items.iter().all(|l| l.endpoint_name == "a"));
    }

    #[test]
    fn prune_removes_old_rows() {
        let mut c = db();
        insert_batch(&mut c, &[log(100, "a", false), log(500, "a", false)], "dev").unwrap();
        let removed = prune_older_than(&c, 300).unwrap();
        assert_eq!(removed, 1);
        let (items, total) = query_page(&c, None, None, None, 50, 0).unwrap();
        assert_eq!(total, 1);
        assert_eq!(items[0].ts, 500);
    }

    #[test]
    fn query_round_trips_path_columns() {
        let mut c = db();
        // 正常行带路径；模拟旧行：路径留空串
        let mut empty = log(200, "b", false);
        empty.inbound_path = String::new();
        empty.upstream_path = String::new();
        insert_batch(&mut c, &[log(100, "a", false), empty], "dev").unwrap();
        let (items, _) = query_page(&c, None, None, None, 50, 0).unwrap();
        // ts 倒序：b(200) 在前，a(100) 在后
        assert_eq!(items[0].inbound_path, "");
        assert_eq!(items[0].upstream_path, "");
        assert_eq!(items[1].inbound_path, "/v1/messages");
        assert_eq!(items[1].upstream_path, "/v1/chat/completions");
    }
}
