use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};

use crate::error::AppResult;
use crate::models::stats::{RequestLog, StatsOverview, TrendCompare};
use crate::modules::stats::periods;
use crate::modules::storage::{db::DbPool, request_logs_repo, stats_repo};
use crate::modules::usage::TokenUsage;

const STATS_EVENT: &str = "stats-updated";
const REQUEST_LOG_EVENT: &str = "request-logged";
const ENDPOINT_HEALTH_EVENT: &str = "endpoint-health-changed";
const FLUSH_INTERVAL: Duration = Duration::from_secs(2);
/// 请求明细保留窗口：90 天。
const RETENTION_MS: i64 = 90 * 24 * 60 * 60 * 1000;
/// 明细清理最小间隔：每小时至多一次。
const PRUNE_INTERVAL: Duration = Duration::from_secs(3600);

#[derive(Default, Clone, Copy)]
struct Delta {
    requests: i64,
    errors: i64,
    input_tokens: i64,
    output_tokens: i64,
    cache_creation_tokens: i64,
    cache_read_tokens: i64,
}

/// 一次请求结果的完整记录（由代理转发汇聚点构造）。
pub struct RequestRecord {
    pub endpoint_name: String,
    pub model: Option<String>,
    pub inbound_format: String,
    pub upstream_url: String,
    /// 真实入站路由路径（`uri.path()`）。
    pub inbound_path: String,
    /// 真实出站路由路径（实际转发上游的路径）。失败兜底为空串。
    pub upstream_path: String,
    pub status_code: Option<i64>,
    pub is_error: bool,
    pub usage: TokenUsage,
    pub duration_ms: Option<i64>,
    pub first_byte_ms: Option<i64>,
    pub actual_model: Option<String>,
    pub error_body: Option<String>,
}

/// 统计聚合器：内存累加 + 2 秒防抖批量落库 + 零延迟事件推送。
///
/// `record` 累加内存（按日聚合 + 明细缓冲）并立即发 `stats-updated` / `request-logged` 事件；
/// DB 写入由 2s 刷新循环或 `overview`（flush-then-read）触发，避免每请求都写库。
pub struct StatsAggregator {
    db_pool: DbPool,
    app_handle: AppHandle,
    device_id: String,
    pending: Mutex<HashMap<(String, String), Delta>>,
    pending_logs: Mutex<Vec<RequestLog>>,
    last_prune: Mutex<Option<Instant>>,
}

impl StatsAggregator {
    pub fn new(db_pool: DbPool, app_handle: AppHandle, device_id: String) -> Arc<Self> {
        let agg = Arc::new(Self {
            db_pool,
            app_handle,
            device_id,
            pending: Mutex::new(HashMap::new()),
            pending_logs: Mutex::new(Vec::new()),
            last_prune: Mutex::new(None),
        });
        // 2 秒防抖刷新循环；聚合器被释放后自动退出
        let weak = Arc::downgrade(&agg);
        tauri::async_runtime::spawn(async move {
            let mut tick = tokio::time::interval(FLUSH_INTERVAL);
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tick.tick().await;
                match weak.upgrade() {
                    Some(a) => {
                        if let Err(e) = a.flush() {
                            tracing::warn!("统计刷新失败: {e}");
                        }
                    }
                    None => break,
                }
            }
        });
        agg
    }

    /// 端点健康/熔断状态变化事件（前端收到后重新拉取 `get_endpoint_health`）。
    pub fn emit_health_changed(&self) {
        let _ = self.app_handle.emit(ENDPOINT_HEALTH_EVENT, ());
    }

    /// 记录一次请求结果（累加内存 + 缓冲明细 + 立即发事件）。
    pub fn record(&self, rec: RequestRecord) {
        let date = periods::today();
        let ts = chrono::Utc::now().timestamp_millis();
        {
            let mut p = self.pending.lock().unwrap();
            let d = p.entry((rec.endpoint_name.clone(), date)).or_default();
            d.requests += 1;
            if rec.is_error {
                d.errors += 1;
            }
            d.input_tokens += rec.usage.input;
            d.output_tokens += rec.usage.output;
            d.cache_creation_tokens += rec.usage.cache_creation;
            d.cache_read_tokens += rec.usage.cache_read;
        }
        let log = RequestLog {
            id: 0,
            ts,
            endpoint_name: rec.endpoint_name,
            inbound_format: rec.inbound_format,
            upstream_url: rec.upstream_url,
            inbound_path: rec.inbound_path,
            upstream_path: rec.upstream_path,
            status_code: rec.status_code,
            is_error: rec.is_error,
            input_tokens: rec.usage.input,
            output_tokens: rec.usage.output,
            cache_creation_tokens: rec.usage.cache_creation,
            cache_read_tokens: rec.usage.cache_read,
            model: rec.model,
            duration_ms: rec.duration_ms,
            first_byte_ms: rec.first_byte_ms,
            actual_model: rec.actual_model,
            error_body: rec.error_body,
        };
        {
            let mut pl = self.pending_logs.lock().unwrap();
            pl.push(log.clone());
        }
        let _ = self.app_handle.emit(STATS_EVENT, ());
        let _ = self.app_handle.emit(REQUEST_LOG_EVENT, &log);
    }

    fn should_prune(&self) -> bool {
        match *self.last_prune.lock().unwrap() {
            None => true,
            Some(t) => t.elapsed() >= PRUNE_INTERVAL,
        }
    }

    fn mark_pruned(&self) {
        *self.last_prune.lock().unwrap() = Some(Instant::now());
    }

    /// 将内存增量批量写入 DB（幂等：无增量且无需清理时直接返回）。
    pub fn flush(&self) -> AppResult<()> {
        let drained: Vec<((String, String), Delta)> = {
            let mut p = self.pending.lock().unwrap();
            p.drain().collect()
        };
        let drained_logs: Vec<RequestLog> = {
            let mut pl = self.pending_logs.lock().unwrap();
            pl.drain(..).collect()
        };
        let prune = self.should_prune();
        if drained.is_empty() && drained_logs.is_empty() && !prune {
            return Ok(());
        }
        let mut conn = self.db_pool.get()?;
        for ((endpoint, date), d) in drained {
            stats_repo::upsert(
                &conn,
                &endpoint,
                &date,
                &self.device_id,
                d.requests,
                d.errors,
                d.input_tokens,
                d.output_tokens,
                d.cache_creation_tokens,
                d.cache_read_tokens,
            )?;
        }
        if !drained_logs.is_empty() {
            request_logs_repo::insert_batch(&mut conn, &drained_logs, &self.device_id)?;
        }
        if prune {
            let cutoff = chrono::Utc::now().timestamp_millis() - RETENTION_MS;
            if let Err(e) = request_logs_repo::prune_older_than(&conn, cutoff) {
                tracing::warn!("请求明细清理失败: {e}");
            }
            self.mark_pruned();
        }
        Ok(())
    }

    /// 四周期总览 + 趋势（先 flush 保证数据完整，再查 DB）。
    pub fn overview(&self) -> AppResult<StatsOverview> {
        self.flush()?;
        let conn = self.db_pool.get()?;
        let t = periods::today_range();
        let y = periods::yesterday_range();
        let w = periods::this_week_range();
        let m = periods::this_month_range();
        let today = stats_repo::period_stats(&conn, &t.start, &t.end)?;
        let yesterday = stats_repo::period_stats(&conn, &y.start, &y.end)?;
        let this_week = stats_repo::period_stats(&conn, &w.start, &w.end)?;
        let this_month = stats_repo::period_stats(&conn, &m.start, &m.end)?;
        let trend = TrendCompare {
            requests_pct: periods::calculate_trend(today.requests, yesterday.requests),
            input_tokens_pct: periods::calculate_trend(today.input_tokens, yesterday.input_tokens),
            output_tokens_pct: periods::calculate_trend(
                today.output_tokens,
                yesterday.output_tokens,
            ),
        };
        Ok(StatsOverview {
            today,
            yesterday,
            this_week,
            this_month,
            trend,
        })
    }
}
