use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendCompare {
    pub requests_pct: f64,
    pub input_tokens_pct: f64,
    pub output_tokens_pct: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsOverview {
    pub today: PeriodStats,
    pub yesterday: PeriodStats,
    pub this_week: PeriodStats,
    pub this_month: PeriodStats,
    pub trend: TrendCompare,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequestTraceHeader {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequestTraceStage {
    pub method: Option<String>,
    pub url: Option<String>,
    pub status_code: Option<i64>,
    pub headers: Vec<RequestTraceHeader>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RequestTrace {
    pub received_request: RequestTraceStage,
    pub forward_request: RequestTraceStage,
    pub received_forwarded_request: RequestTraceStage,
    pub response_request: RequestTraceStage,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestLog {
    pub id: i64,
    pub ts: i64,
    pub endpoint_name: String,
    pub inbound_format: String,
    /// 端点 transformer 快照（claude/openai/codex 等）。旧行/未记录为 None，前端回退 inbound_format。
    pub transformer: Option<String>,
    pub upstream_url: String,
    pub inbound_path: String,
    pub upstream_path: String,
    pub status_code: Option<i64>,
    pub is_error: bool,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_creation_tokens: i64,
    pub cache_read_tokens: i64,
    pub model: Option<String>,
    pub duration_ms: Option<i64>,
    pub first_byte_ms: Option<i64>,
    pub actual_model: Option<String>,
    pub error_body: Option<String>,
    pub trace: Option<RequestTrace>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestLogPage {
    pub items: Vec<RequestLog>,
    pub total: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsHistoryPage {
    pub items: Vec<DailyStat>,
    pub total: i64,
}
