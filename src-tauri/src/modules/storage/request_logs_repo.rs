use rusqlite::{params, params_from_iter, types::Value as SqlValue, Connection};

use crate::error::AppResult;
use crate::models::stats::{RequestLog, RequestTrace};

pub fn insert_batch(conn: &mut Connection, logs: &[RequestLog], device_id: &str) -> AppResult<()> {
    if logs.is_empty() {
        return Ok(());
    }

    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare(
            "INSERT INTO request_logs(
                ts, endpoint_name, inbound_format, transformer, upstream_url, inbound_path, upstream_path,
                status_code, is_error, input_tokens, output_tokens, cache_creation_tokens,
                cache_read_tokens, model, duration_ms, first_byte_ms, actual_model, error_body,
                trace_detail, device_id)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
        )?;
        for log in logs {
            let trace_json = log
                .trace
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?;
            stmt.execute(params![
                log.ts,
                log.endpoint_name,
                log.inbound_format,
                log.transformer,
                log.upstream_url,
                log.inbound_path,
                log.upstream_path,
                log.status_code,
                log.is_error as i64,
                log.input_tokens,
                log.output_tokens,
                log.cache_creation_tokens,
                log.cache_read_tokens,
                log.model,
                log.duration_ms,
                log.first_byte_ms,
                log.actual_model,
                log.error_body,
                trace_json,
                device_id,
            ])?;
        }
    }
    tx.commit()?;
    Ok(())
}

fn row_to_log(row: &rusqlite::Row) -> rusqlite::Result<RequestLog> {
    let trace_json: Option<String> = row.get(19)?;
    let trace = trace_json
        .as_deref()
        .map(serde_json::from_str::<RequestTrace>)
        .transpose()
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                19,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;

    Ok(RequestLog {
        id: row.get(0)?,
        ts: row.get(1)?,
        endpoint_name: row.get(2)?,
        inbound_format: row.get(3)?,
        transformer: row.get(4)?,
        upstream_url: row.get(5)?,
        status_code: row.get(6)?,
        is_error: row.get::<_, i64>(7)? != 0,
        input_tokens: row.get(8)?,
        output_tokens: row.get(9)?,
        cache_creation_tokens: row.get(10)?,
        cache_read_tokens: row.get(11)?,
        model: row.get(12)?,
        duration_ms: row.get(13)?,
        inbound_path: row.get(14)?,
        upstream_path: row.get(15)?,
        first_byte_ms: row.get(16)?,
        actual_model: row.get(17)?,
        error_body: row.get(18)?,
        trace,
    })
}

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
    if let Some(start_ms) = start_ms {
        where_sql.push_str(" AND ts >= ?");
        args.push(SqlValue::Integer(start_ms));
    }
    if let Some(end_ms) = end_ms {
        where_sql.push_str(" AND ts <= ?");
        args.push(SqlValue::Integer(end_ms));
    }
    if let Some(endpoint) = endpoint {
        if !endpoint.is_empty() {
            where_sql.push_str(" AND endpoint_name = ?");
            args.push(SqlValue::Text(endpoint.to_string()));
        }
    }

    let total: i64 = {
        let sql = format!("SELECT COUNT(*) FROM request_logs{where_sql}");
        conn.query_row(&sql, params_from_iter(args.iter()), |row| row.get(0))?
    };

    let mut page_args = args.clone();
    page_args.push(SqlValue::Integer(limit));
    page_args.push(SqlValue::Integer(offset));
    let sql = format!(
        "SELECT id, ts, endpoint_name, inbound_format, transformer, upstream_url, status_code, is_error,
                input_tokens, output_tokens, cache_creation_tokens, cache_read_tokens, model, duration_ms,
                inbound_path, upstream_path, first_byte_ms, actual_model, error_body, trace_detail
         FROM request_logs{where_sql}
         ORDER BY ts DESC, id DESC LIMIT ? OFFSET ?"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(page_args.iter()), row_to_log)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok((items, total))
}

pub fn prune_older_than(conn: &Connection, cutoff_ms: i64) -> AppResult<usize> {
    let removed = conn.execute("DELETE FROM request_logs WHERE ts < ?1", params![cutoff_ms])?;
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::stats::{RequestTraceHeader, RequestTraceStage};
    use crate::modules::storage::migration::run_migrations;

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();
        conn
    }

    fn sample_trace(label: &str) -> RequestTrace {
        RequestTrace {
            received_request: RequestTraceStage {
                method: Some("POST".to_string()),
                url: Some("/v1/messages".to_string()),
                status_code: None,
                headers: vec![RequestTraceHeader {
                    key: "content-type".to_string(),
                    value: "application/json".to_string(),
                }],
                body: Some(format!(r#"{{"stage":"{label}-received"}}"#)),
            },
            forward_request: RequestTraceStage {
                method: Some("POST".to_string()),
                url: Some("https://up.example/v1/chat/completions".to_string()),
                status_code: None,
                headers: vec![RequestTraceHeader {
                    key: "authorization".to_string(),
                    value: "[redacted]".to_string(),
                }],
                body: Some(format!(r#"{{"stage":"{label}-forward"}}"#)),
            },
            received_forwarded_request: RequestTraceStage {
                method: None,
                url: Some("https://up.example/v1/chat/completions".to_string()),
                status_code: Some(200),
                headers: vec![RequestTraceHeader {
                    key: "content-type".to_string(),
                    value: "application/json".to_string(),
                }],
                body: Some(format!(r#"{{"stage":"{label}-upstream-response"}}"#)),
            },
            response_request: RequestTraceStage {
                method: None,
                url: Some("/v1/messages".to_string()),
                status_code: Some(200),
                headers: vec![RequestTraceHeader {
                    key: "content-type".to_string(),
                    value: "application/json".to_string(),
                }],
                body: Some(format!(r#"{{"stage":"{label}-response"}}"#)),
            },
        }
    }

    fn log(ts: i64, endpoint: &str, is_error: bool) -> RequestLog {
        RequestLog {
            id: 0,
            ts,
            endpoint_name: endpoint.to_string(),
            inbound_format: "claude".to_string(),
            transformer: Some("claude".to_string()),
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
            first_byte_ms: Some(45),
            actual_model: None,
            error_body: None,
            trace: Some(sample_trace(endpoint)),
        }
    }

    #[test]
    fn insert_and_query_paginates_desc() {
        let mut conn = db();
        insert_batch(
            &mut conn,
            &[
                log(100, "a", false),
                log(200, "b", true),
                log(300, "a", false),
            ],
            "dev",
        )
        .unwrap();

        let (page1, total) = query_page(&conn, None, None, None, 2, 0).unwrap();
        assert_eq!(total, 3);
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].ts, 300);
        assert_eq!(page1[1].ts, 200);

        let (page2, _) = query_page(&conn, None, None, None, 2, 2).unwrap();
        assert_eq!(page2.len(), 1);
        assert_eq!(page2[0].ts, 100);
        assert!(page1[1].is_error);
        assert_eq!(page1[0].cache_read_tokens, 2);
        assert_eq!(page1[0].first_byte_ms, Some(45));
        assert_eq!(page1[0].actual_model, None);
    }

    #[test]
    fn actual_model_roundtrips() {
        let mut conn = db();
        let mut mapped = log(100, "a", false);
        mapped.actual_model = Some("gpt-5.5".to_string());
        insert_batch(&mut conn, &[mapped, log(200, "b", false)], "dev").unwrap();
        let (items, _) = query_page(&conn, None, None, None, 50, 0).unwrap();
        assert_eq!(items[0].actual_model, None);
        assert_eq!(items[1].actual_model.as_deref(), Some("gpt-5.5"));
    }

    #[test]
    fn query_filters_by_time_and_endpoint() {
        let mut conn = db();
        insert_batch(
            &mut conn,
            &[
                log(100, "a", false),
                log(200, "b", false),
                log(300, "a", false),
            ],
            "dev",
        )
        .unwrap();

        let (items, total) = query_page(&conn, Some(150), Some(350), None, 50, 0).unwrap();
        assert_eq!(total, 2);
        assert_eq!(items.len(), 2);

        let (a_items, a_total) = query_page(&conn, None, None, Some("a"), 50, 0).unwrap();
        assert_eq!(a_total, 2);
        assert!(a_items.iter().all(|item| item.endpoint_name == "a"));
    }

    #[test]
    fn prune_removes_old_rows() {
        let mut conn = db();
        insert_batch(
            &mut conn,
            &[log(100, "a", false), log(500, "a", false)],
            "dev",
        )
        .unwrap();

        let removed = prune_older_than(&conn, 300).unwrap();
        assert_eq!(removed, 1);

        let (items, total) = query_page(&conn, None, None, None, 50, 0).unwrap();
        assert_eq!(total, 1);
        assert_eq!(items[0].ts, 500);
    }

    #[test]
    fn query_round_trips_path_columns() {
        let mut conn = db();
        let mut empty = log(200, "b", false);
        empty.inbound_path = String::new();
        empty.upstream_path = String::new();
        insert_batch(&mut conn, &[log(100, "a", false), empty], "dev").unwrap();

        let (items, _) = query_page(&conn, None, None, None, 50, 0).unwrap();
        assert_eq!(items[0].inbound_path, "");
        assert_eq!(items[0].upstream_path, "");
        assert_eq!(items[1].inbound_path, "/v1/messages");
        assert_eq!(items[1].upstream_path, "/v1/chat/completions");
    }

    #[test]
    fn error_body_roundtrips() {
        let mut conn = db();
        let mut failed = log(100, "a", true);
        failed.status_code = Some(403);
        failed.error_body = Some(r#"{"error":{"code":"channel:client_restricted"}}"#.to_string());
        insert_batch(&mut conn, &[failed], "dev").unwrap();

        let (items, _) = query_page(&conn, None, None, None, 50, 0).unwrap();
        assert_eq!(
            items[0].error_body.as_deref(),
            Some(r#"{"error":{"code":"channel:client_restricted"}}"#)
        );
    }

    #[test]
    fn trace_roundtrips() {
        let mut conn = db();
        let expected = sample_trace("trace");
        let mut traced = log(100, "a", false);
        traced.trace = Some(expected.clone());
        insert_batch(&mut conn, &[traced], "dev").unwrap();

        let (items, _) = query_page(&conn, None, None, None, 50, 0).unwrap();
        assert_eq!(items[0].trace, Some(expected));
    }
}
