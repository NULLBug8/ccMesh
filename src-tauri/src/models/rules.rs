use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RoutingRules {
    pub strategy: String,
    pub model_affinity: bool,
    pub header_affinity: bool,
    pub model_mapping_strategy: String,
    pub max_retries: u32,
    pub request_timeout_seconds: u64,
}

impl Default for RoutingRules {
    fn default() -> Self {
        Self {
            strategy: "balanced".into(),
            model_affinity: true,
            header_affinity: true,
            model_mapping_strategy: "site-first".into(),
            max_retries: 0,
            request_timeout_seconds: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CircuitBreakerRules {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout_seconds: u64,
    pub error_rate_threshold: f64,
    pub min_requests: u32,
    pub failure_status_codes: Vec<u16>,
}

impl Default for CircuitBreakerRules {
    fn default() -> Self {
        Self {
            failure_threshold: 4,
            success_threshold: 2,
            timeout_seconds: 60,
            error_rate_threshold: 0.6,
            min_requests: 10,
            failure_status_codes: vec![429, 500, 502, 503, 504],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DegradationRules {
    pub enabled: bool,
    pub reasoning_effort_fallback: bool,
    pub request_thinking_signature: bool,
    pub retry_without_stream: bool,
    pub fallback_temperature: f64,
}

impl Default for DegradationRules {
    fn default() -> Self {
        Self {
            enabled: true,
            reasoning_effort_fallback: true,
            request_thinking_signature: true,
            retry_without_stream: false,
            fallback_temperature: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RulesConfig {
    pub routing: RoutingRules,
    pub circuit_breaker: CircuitBreakerRules,
    pub degradation: DegradationRules,
}
