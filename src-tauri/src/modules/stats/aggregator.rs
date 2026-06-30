use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::runtime::AppHandle;

use crate::error::AppResult;
use crate::models::stats::{RequestLog, RequestTrace, StatsOverview, TrendCompare};
use crate::modules::stats::periods;
use crate::modules::storage::{db::DbPool, request_logs_repo, stats_repo};
use crate::modules::usage::TokenUsage;

const STATS_EVENT: &str = "stats-updated";
const REQUEST_LOG_EVENT: &str = "request-logged";
const ENDPOINT_HEALTH_EVENT: &str = "endpoint-health-changed";
const FLUSH_INTERVAL: Duration = Duration::from_secs(2);
const RETENTION_MS: i64 = 90 * 24 * 60 * 60 * 1000;
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

pub struct RequestRecord {
    pub endpoint_name: String,
    pub model: Option<String>,
    pub inbound_format: String,
    pub upstream_url: String,
    pub inbound_path: String,
    pub upstream_path: String,
    pub status_code: Option<i64>,
    pub is_error: bool,
    pub usage: TokenUsage,
    pub duration_ms: Option<i64>,
    pub first_byte_ms: Option<i64>,
    pub actual_model: Option<String>,
    pub error_body: Option<String>,
    pub trace: Option<RequestTrace>,
}

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
        let aggregator = Arc::new(Self {
            db_pool,
            app_handle,
            device_id,
            pending: Mutex::new(HashMap::new()),
            pending_logs: Mutex::new(Vec::new()),
            last_prune: Mutex::new(None),
        });

        let weak = Arc::downgrade(&aggregator);
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(FLUSH_INTERVAL);
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tick.tick().await;
                match weak.upgrade() {
                    Some(aggregator) => {
                        if let Err(error) = aggregator.flush() {
                            tracing::warn!("统计刷新失败: {error}");
                        }
                    }
                    None => break,
                }
            }
        });

        aggregator
    }

    pub fn emit_health_changed(&self) {
        let _ = self.app_handle.emit(ENDPOINT_HEALTH_EVENT, ());
        crate::modules::web_admin::bridge::emit(ENDPOINT_HEALTH_EVENT, &());
    }

    pub fn record(&self, record: RequestRecord) {
        let date = periods::today();
        let ts = chrono::Utc::now().timestamp_millis();

        {
            let mut pending = self.pending.lock().unwrap();
            let delta = pending
                .entry((record.endpoint_name.clone(), date))
                .or_default();
            delta.requests += 1;
            if record.is_error {
                delta.errors += 1;
            }
            delta.input_tokens += record.usage.input;
            delta.output_tokens += record.usage.output;
            delta.cache_creation_tokens += record.usage.cache_creation;
            delta.cache_read_tokens += record.usage.cache_read;
        }

        let log = RequestLog {
            id: 0,
            ts,
            endpoint_name: record.endpoint_name,
            inbound_format: record.inbound_format,
            upstream_url: record.upstream_url,
            inbound_path: record.inbound_path,
            upstream_path: record.upstream_path,
            status_code: record.status_code,
            is_error: record.is_error,
            input_tokens: record.usage.input,
            output_tokens: record.usage.output,
            cache_creation_tokens: record.usage.cache_creation,
            cache_read_tokens: record.usage.cache_read,
            model: record.model,
            duration_ms: record.duration_ms,
            first_byte_ms: record.first_byte_ms,
            actual_model: record.actual_model,
            error_body: record.error_body,
            trace: record.trace,
        };

        {
            let mut pending_logs = self.pending_logs.lock().unwrap();
            pending_logs.push(log.clone());
        }

        let _ = self.app_handle.emit(STATS_EVENT, ());
        let _ = self.app_handle.emit(REQUEST_LOG_EVENT, &log);
        crate::modules::web_admin::bridge::emit(STATS_EVENT, &());
        crate::modules::web_admin::bridge::emit(REQUEST_LOG_EVENT, &log);
    }

    fn should_prune(&self) -> bool {
        match *self.last_prune.lock().unwrap() {
            None => true,
            Some(last_prune) => last_prune.elapsed() >= PRUNE_INTERVAL,
        }
    }

    fn mark_pruned(&self) {
        *self.last_prune.lock().unwrap() = Some(Instant::now());
    }

    pub fn flush(&self) -> AppResult<()> {
        let drained: Vec<((String, String), Delta)> = {
            let mut pending = self.pending.lock().unwrap();
            pending.drain().collect()
        };
        let drained_logs: Vec<RequestLog> = {
            let mut pending_logs = self.pending_logs.lock().unwrap();
            pending_logs.drain(..).collect()
        };
        let should_prune = self.should_prune();

        if drained.is_empty() && drained_logs.is_empty() && !should_prune {
            return Ok(());
        }

        let mut conn = self.db_pool.get()?;
        for ((endpoint, date), delta) in drained {
            stats_repo::upsert(
                &conn,
                &endpoint,
                &date,
                &self.device_id,
                delta.requests,
                delta.errors,
                delta.input_tokens,
                delta.output_tokens,
                delta.cache_creation_tokens,
                delta.cache_read_tokens,
            )?;
        }

        if !drained_logs.is_empty() {
            request_logs_repo::insert_batch(&mut conn, &drained_logs, &self.device_id)?;
        }

        if should_prune {
            let cutoff = chrono::Utc::now().timestamp_millis() - RETENTION_MS;
            if let Err(error) = request_logs_repo::prune_older_than(&conn, cutoff) {
                tracing::warn!("请求明细清理失败: {error}");
            }
            self.mark_pruned();
        }

        Ok(())
    }

    pub fn overview(&self) -> AppResult<StatsOverview> {
        self.flush()?;
        let conn = self.db_pool.get()?;
        let today_range = periods::today_range();
        let yesterday_range = periods::yesterday_range();
        let week_range = periods::this_week_range();
        let month_range = periods::this_month_range();
        let today = stats_repo::period_stats(&conn, &today_range.start, &today_range.end)?;
        let yesterday = stats_repo::period_stats(&conn, &yesterday_range.start, &yesterday_range.end)?;
        let this_week = stats_repo::period_stats(&conn, &week_range.start, &week_range.end)?;
        let this_month = stats_repo::period_stats(&conn, &month_range.start, &month_range.end)?;
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
