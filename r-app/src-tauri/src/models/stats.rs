use serde::{Deserialize, Serialize};

/// 单端点单日统计行（对应 `daily_stats`）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyStat {
    pub endpoint_name: String,
    pub date: String,
    pub requests: i64,
    pub errors: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

/// 某端点在一个周期内的聚合。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndpointStat {
    pub endpoint_name: String,
    pub requests: i64,
    pub errors: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

/// 单周期聚合（总量 + 每端点明细）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeriodStats {
    pub requests: i64,
    pub errors: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub endpoints: Vec<EndpointStat>,
}

/// 趋势对比（今日 vs 昨日的百分比变化）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendCompare {
    pub requests_pct: f64,
    pub input_tokens_pct: f64,
    pub output_tokens_pct: f64,
}

/// 四周期统计总览 + 趋势（`get_stats` 返回，`stats-updated` 事件推送）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsOverview {
    pub today: PeriodStats,
    pub yesterday: PeriodStats,
    pub this_week: PeriodStats,
    pub this_month: PeriodStats,
    pub trend: TrendCompare,
}
