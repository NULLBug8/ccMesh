use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tauri::{AppHandle, Emitter};

use crate::error::AppResult;
use crate::models::stats::{StatsOverview, TrendCompare};
use crate::modules::stats::periods;
use crate::modules::storage::{db::DbPool, stats_repo};

const STATS_EVENT: &str = "stats-updated";
const FLUSH_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Default, Clone, Copy)]
struct Delta {
    requests: i64,
    errors: i64,
    input_tokens: i64,
    output_tokens: i64,
}

/// 统计聚合器：内存累加 + 2 秒防抖批量落库 + 零延迟事件推送。
///
/// `record` 仅累加内存并立即发 `stats-updated` 事件；DB 写入由 2s 刷新循环或
/// `overview`（flush-then-read）触发，避免每请求都写库。
pub struct StatsAggregator {
    db_pool: DbPool,
    app_handle: AppHandle,
    device_id: String,
    pending: Mutex<HashMap<(String, String), Delta>>,
}

impl StatsAggregator {
    pub fn new(db_pool: DbPool, app_handle: AppHandle, device_id: String) -> Arc<Self> {
        let agg = Arc::new(Self {
            db_pool,
            app_handle,
            device_id,
            pending: Mutex::new(HashMap::new()),
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

    /// 记录一次请求结果（累加内存 + 立即发事件）。
    pub fn record(&self, endpoint_name: &str, is_error: bool, input_tokens: i64, output_tokens: i64) {
        let date = periods::today();
        {
            let mut p = self.pending.lock().unwrap();
            let d = p.entry((endpoint_name.to_string(), date)).or_default();
            d.requests += 1;
            if is_error {
                d.errors += 1;
            }
            d.input_tokens += input_tokens;
            d.output_tokens += output_tokens;
        }
        let _ = self.app_handle.emit(STATS_EVENT, ());
    }

    /// 将内存增量批量写入 DB（幂等：无增量时直接返回）。
    pub fn flush(&self) -> AppResult<()> {
        let drained: Vec<((String, String), Delta)> = {
            let mut p = self.pending.lock().unwrap();
            if p.is_empty() {
                return Ok(());
            }
            p.drain().collect()
        };
        let conn = self.db_pool.get()?;
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
            )?;
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
            output_tokens_pct: periods::calculate_trend(today.output_tokens, yesterday.output_tokens),
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
