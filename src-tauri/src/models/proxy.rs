use serde::{Deserialize, Serialize};

/// 代理运行状态：`get_proxy_status` 返回值，并经 `proxy-status-changed` 事件推送前端。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStatus {
    pub running: bool,
    pub port: u16,
    pub current_endpoint: Option<String>,
    pub enabled_endpoint_count: usize,
}

impl Default for ProxyStatus {
    fn default() -> Self {
        Self {
            running: false,
            port: 0,
            current_endpoint: None,
            enabled_endpoint_count: 0,
        }
    }
}
