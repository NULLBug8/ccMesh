use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RoutingRules {
    pub strategy: String,
    pub model_affinity: bool,
    pub header_affinity: bool,
}

impl Default for RoutingRules {
    fn default() -> Self {
        Self {
            strategy: "balanced".into(),
            model_affinity: true,
            header_affinity: true,
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
}

impl Default for CircuitBreakerRules {
    fn default() -> Self {
        Self {
            failure_threshold: 4,
            success_threshold: 2,
            timeout_seconds: 60,
            error_rate_threshold: 0.6,
            min_requests: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DegradationRules {
    pub enabled: bool,
    pub reasoning_effort_fallback: bool,
    pub request_thinking_signature: bool,
}

impl Default for DegradationRules {
    fn default() -> Self {
        Self {
            enabled: true,
            reasoning_effort_fallback: true,
            request_thinking_signature: true,
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
