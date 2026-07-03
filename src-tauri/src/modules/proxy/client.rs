//! 代理决策与 HTTP client 构建：统一「转发 / 获取模型 / 测试 / 更新」的代理判定。
//!
//! 真值表（地址非空时）：转发/获取模型 `proxy = use_proxy || proxy_enabled`；
//! 更新 `proxy = proxy_for_update`。地址为空 → 一律直连。

use std::time::Duration;

use crate::error::{AppError, AppResult};

/// 转发 / 获取模型是否走代理：端点 `use_proxy` 或全局 `proxy_enabled`，且代理地址非空。
pub fn should_use_proxy(use_proxy: bool, proxy_enabled: bool, proxy_url: &str) -> bool {
    (use_proxy || proxy_enabled) && !proxy_url.trim().is_empty()
}

/// 应用更新是否走代理：`proxy_for_update` 且代理地址非空。
pub fn should_proxy_update(proxy_for_update: bool, proxy_url: &str) -> bool {
    proxy_for_update && !proxy_url.trim().is_empty()
}

/// 构建 reqwest client：`want_proxy` 且地址有效 → 经代理；否则显式直连（`no_proxy`，忽略系统环境代理）。
/// 地址无效时 warn 后回落直连（不再静默）。
pub fn build_client(
    want_proxy: bool,
    proxy_url: &str,
    timeout: Duration,
) -> AppResult<reqwest::Client> {
    let mut b = reqwest::Client::builder().timeout(timeout);
    let url = proxy_url.trim();
    if want_proxy && !url.is_empty() {
        match reqwest::Proxy::all(url) {
            Ok(p) => b = b.proxy(p),
            Err(e) => {
                tracing::warn!("代理地址无效，回落直连: {e}");
                b = b.no_proxy();
            }
        }
    } else {
        b = b.no_proxy();
    }
    b.build()
        .map_err(|e| AppError::Proxy(format!("构建 HTTP 客户端失败: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_proxy_truth_table() {
        // 地址空 → 永远直连
        assert!(!should_use_proxy(true, true, ""));
        assert!(!should_use_proxy(true, false, "   "));
        // 端点开 或 全局开 → 走代理
        assert!(should_use_proxy(true, false, "http://p"));
        assert!(should_use_proxy(false, true, "http://p"));
        assert!(should_use_proxy(true, true, "http://p"));
        // 都关 → 直连
        assert!(!should_use_proxy(false, false, "http://p"));
    }

    #[test]
    fn proxy_update_truth_table() {
        assert!(should_proxy_update(true, "http://p"));
        assert!(!should_proxy_update(false, "http://p"));
        assert!(!should_proxy_update(true, ""));
        assert!(!should_proxy_update(true, "   "));
    }

    #[test]
    fn build_client_succeeds_in_all_modes() {
        assert!(build_client(true, "http://127.0.0.1:7890", Duration::from_secs(5)).is_ok());
        assert!(build_client(false, "", Duration::from_secs(5)).is_ok());
        // 无效地址回落直连仍能构建
        assert!(build_client(true, "http://", Duration::from_secs(5)).is_ok());
    }
}
