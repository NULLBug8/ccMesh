use serde::{Deserialize, Serialize};

use crate::models::rules::RulesConfig;
use crate::utils::ua;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub port: u16,
    pub log_level: String,
    pub language: String,
    pub theme: String,
    pub theme_auto: bool,
    pub auto_light_start: String,
    pub auto_dark_start: String,
    pub auto_run: bool,
    pub models_cache_ttl: i64,
    pub proxy_url: String,
    pub proxy_enabled: bool,
    pub openai_ua: String,
    pub claude_cli_ua: String,
    pub rules: RulesConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            port: 3000,
            log_level: "info".into(),
            language: "zh".into(),
            theme: "system".into(),
            theme_auto: false,
            auto_light_start: "07:00".into(),
            auto_dark_start: "19:00".into(),
            auto_run: true,
            models_cache_ttl: 30,
            proxy_url: String::new(),
            proxy_enabled: false,
            openai_ua: ua::codex_probe_ua(),
            claude_cli_ua: ua::CLAUDE_PROBE_UA.into(),
            rules: RulesConfig::default(),
        }
    }
}

pub fn port_with_env_override(port: u16) -> u16 {
    std::env::var("CCMESH_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(port)
}

pub fn auto_run_with_env_override(auto_run: bool) -> bool {
    auto_run || std::env::var_os("CCMESH_PORT").is_some()
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, OnceLock};

    use super::{auto_run_with_env_override, port_with_env_override};

    fn with_env_port<T>(value: Option<&str>, test: impl FnOnce() -> T) -> T {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap();
        let previous = std::env::var("CCMESH_PORT").ok();
        match value {
            Some(value) => std::env::set_var("CCMESH_PORT", value),
            None => std::env::remove_var("CCMESH_PORT"),
        }
        let result = test();
        match previous {
            Some(value) => std::env::set_var("CCMESH_PORT", value),
            None => std::env::remove_var("CCMESH_PORT"),
        }
        result
    }

    #[test]
    fn auto_run_override_starts_when_port_env_is_present() {
        with_env_port(Some("3001"), || {
            assert!(auto_run_with_env_override(false));
        });
    }

    #[test]
    fn auto_run_override_keeps_config_without_port_env() {
        with_env_port(None, || {
            assert!(!auto_run_with_env_override(false));
            assert!(auto_run_with_env_override(true));
        });
    }

    #[test]
    fn port_override_uses_env_when_valid() {
        with_env_port(Some("3001"), || {
            assert_eq!(port_with_env_override(3000), 3001);
        });
    }

    #[test]
    fn port_override_ignores_invalid_env() {
        with_env_port(Some("not-a-port"), || {
            assert_eq!(port_with_env_override(3000), 3000);
        });
    }

    #[test]
    fn port_override_keeps_configured_port_without_env() {
        with_env_port(None, || {
            assert_eq!(port_with_env_override(3000), 3000);
        });
    }
}
