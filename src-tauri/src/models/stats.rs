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
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
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
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
}

/// 单周期聚合（总量 + 每端点明细）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeriodStats {
    pub requests: i64,
    pub errors: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
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

/// 逐条请求明细（对应 `request_logs`）。事件推送时 `id` 为 0（尚未落库）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestLog {
    pub id: i64,
    /// 请求时间（Unix 毫秒，UTC）。
    pub ts: i64,
    pub endpoint_name: String,
    pub inbound_format: String,
    pub upstream_url: String,
    /// 真实入站路由路径（如 `/v1/messages`、`/v1/chat/completions`）。旧行为空串。
    pub inbound_path: String,
    /// 真实出站路由路径（实际转发上游的路径，转换后为 `/v1/chat/completions`）。旧行为空串。
    pub upstream_path: String,
    pub status_code: Option<i64>,
    pub is_error: bool,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
    pub model: Option<String>,
    pub duration_ms: Option<i64>,
}

/// 请求明细分页结果。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestLogPage {
    pub items: Vec<RequestLog>,
    pub total: i64,
}

/// 历史记录分页结果（按端点×日聚合行）。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsHistoryPage {
    pub items: Vec<DailyStat>,
    pub total: i64,
}
