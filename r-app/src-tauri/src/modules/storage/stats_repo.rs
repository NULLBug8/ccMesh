use rusqlite::{params, Connection};

use crate::error::AppResult;
use crate::models::stats::{DailyStat, EndpointStat, PeriodStats};

/// 累加写入一行（UPSERT，按 endpoint_name+date+device_id 累加）。
pub fn upsert(
    conn: &Connection,
    endpoint_name: &str,
    date: &str,
    device_id: &str,
    requests: i64,
    errors: i64,
    input_tokens: i64,
    output_tokens: i64,
) -> AppResult<()> {
    conn.execute(
        "INSERT INTO daily_stats(endpoint_name,date,requests,errors,input_tokens,output_tokens,device_id)
         VALUES(?1,?2,?3,?4,?5,?6,?7)
         ON CONFLICT(endpoint_name,date,device_id) DO UPDATE SET
            requests      = requests + excluded.requests,
            errors        = errors + excluded.errors,
            input_tokens  = input_tokens + excluded.input_tokens,
            output_tokens = output_tokens + excluded.output_tokens",
        params![endpoint_name, date, requests, errors, input_tokens, output_tokens, device_id],
    )?;
    Ok(())
}

/// 聚合某日期范围（闭区间）内每端点统计 + 周期总量。
pub fn period_stats(conn: &Connection, start: &str, end: &str) -> AppResult<PeriodStats> {
    let mut stmt = conn.prepare(
        "SELECT endpoint_name, SUM(requests), SUM(errors), SUM(input_tokens), SUM(output_tokens)
         FROM daily_stats WHERE date >= ?1 AND date <= ?2
         GROUP BY endpoint_name ORDER BY endpoint_name",
    )?;
    let rows = stmt.query_map(params![start, end], |r| {
        Ok(EndpointStat {
            endpoint_name: r.get(0)?,
            requests: r.get::<_, Option<i64>>(1)?.unwrap_or(0),
            errors: r.get::<_, Option<i64>>(2)?.unwrap_or(0),
            input_tokens: r.get::<_, Option<i64>>(3)?.unwrap_or(0),
            output_tokens: r.get::<_, Option<i64>>(4)?.unwrap_or(0),
        })
    })?;
    let mut ps = PeriodStats::default();
    for er in rows {
        let e = er?;
        ps.requests += e.requests;
        ps.errors += e.errors;
        ps.input_tokens += e.input_tokens;
        ps.output_tokens += e.output_tokens;
        ps.endpoints.push(e);
    }
    Ok(ps)
}

/// 列出有数据的归档月份（"YYYY-MM" 倒序）。
pub fn archive_months(conn: &Connection) -> AppResult<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT strftime('%Y-%m', date) AS m FROM daily_stats
         WHERE date IS NOT NULL AND date != '' ORDER BY m DESC",
    )?;
    let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// 某月每端点每日明细。
pub fn monthly_data(conn: &Connection, month: &str) -> AppResult<Vec<DailyStat>> {
    let mut stmt = conn.prepare(
        "SELECT endpoint_name, date, SUM(requests), SUM(errors), SUM(input_tokens), SUM(output_tokens)
         FROM daily_stats WHERE strftime('%Y-%m', date) = ?1
         GROUP BY endpoint_name, date ORDER BY date DESC, endpoint_name",
    )?;
    let rows = stmt.query_map(params![month], |r| {
        Ok(DailyStat {
            endpoint_name: r.get(0)?,
            date: r.get(1)?,
            requests: r.get::<_, Option<i64>>(2)?.unwrap_or(0),
            errors: r.get::<_, Option<i64>>(3)?.unwrap_or(0),
            input_tokens: r.get::<_, Option<i64>>(4)?.unwrap_or(0),
            output_tokens: r.get::<_, Option<i64>>(5)?.unwrap_or(0),
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}

/// 删除某月全部统计。返回删除行数。
pub fn delete_month(conn: &Connection, month: &str) -> AppResult<usize> {
    let n = conn.execute(
        "DELETE FROM daily_stats WHERE strftime('%Y-%m', date) = ?1",
        params![month],
    )?;
    Ok(n)
}
