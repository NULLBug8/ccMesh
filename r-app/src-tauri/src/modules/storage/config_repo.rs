use std::collections::BTreeMap;

use rusqlite::{params, Connection, OptionalExtension};

use crate::error::AppResult;
use crate::models::config::{AppConfig, UpdateSettings, WebDavConfig};

/// 可跨设备同步的安全配置键白名单（剔除设备/路径特定项，见 P5-4）。
pub const SAFE_CONFIG_KEYS: &[&str] = &[
    "port",
    "logLevel",
    "language",
    "theme",
    "themeAuto",
    "autoLightStart",
    "autoDarkStart",
    "closeWindowBehavior",
    "modelsCacheTtl",
    "webdav_url",
    "webdav_username",
    "webdav_password",
    "webdav_configPath",
    "webdav_statsPath",
    "update_autoCheck",
    "update_checkInterval",
    "openaiUa",
    "claudeCliUa",
];

pub fn get_value(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM app_config WHERE key = ?1", [key], |r| {
            r.get(0)
        })
        .optional()?)
}

pub fn set_value(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO app_config(key, value) VALUES(?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn get_all(conn: &Connection) -> AppResult<BTreeMap<String, String>> {
    let mut stmt = conn.prepare("SELECT key, value FROM app_config")?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    let mut m = BTreeMap::new();
    for r in rows {
        let (k, v) = r?;
        m.insert(k, v);
    }
    Ok(m)
}

fn parse_bool(m: &BTreeMap<String, String>, key: &str, default: bool) -> bool {
    m.get(key)
        .map(|v| v == "true" || v == "1")
        .unwrap_or(default)
}
fn parse_str(m: &BTreeMap<String, String>, key: &str, default: &str) -> String {
    m.get(key)
        .filter(|v| !v.is_empty())
        .cloned()
        .unwrap_or_else(|| default.to_string())
}
fn parse_i64(m: &BTreeMap<String, String>, key: &str, default: i64) -> i64 {
    m.get(key).and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// 组装强类型 `AppConfig`（缺省回落默认值）。
pub fn get_config(conn: &Connection) -> AppResult<AppConfig> {
    let m = get_all(conn)?;
    let d = AppConfig::default();
    Ok(AppConfig {
        port: parse_i64(&m, "port", d.port as i64) as u16,
        log_level: parse_str(&m, "logLevel", &d.log_level),
        language: parse_str(&m, "language", &d.language),
        theme: parse_str(&m, "theme", &d.theme),
        theme_auto: parse_bool(&m, "themeAuto", d.theme_auto),
        auto_light_start: parse_str(&m, "autoLightStart", &d.auto_light_start),
        auto_dark_start: parse_str(&m, "autoDarkStart", &d.auto_dark_start),
        close_window_behavior: parse_str(&m, "closeWindowBehavior", &d.close_window_behavior),
        models_cache_ttl: parse_i64(&m, "modelsCacheTtl", d.models_cache_ttl),
        proxy_url: parse_str(&m, "proxyUrl", &d.proxy_url),
        proxy_enabled: parse_bool(&m, "proxyEnabled", d.proxy_enabled),
        proxy_for_update: parse_bool(&m, "proxyForUpdate", d.proxy_for_update),
        openai_ua: parse_str(&m, "openaiUa", &d.openai_ua),
        claude_cli_ua: parse_str(&m, "claudeCliUa", &d.claude_cli_ua),
        update: UpdateSettings {
            auto_check: parse_bool(&m, "update_autoCheck", true),
            check_interval: parse_i64(&m, "update_checkInterval", 24),
            skipped_version: parse_str(&m, "update_skippedVersion", ""),
            last_check_time: parse_str(&m, "update_lastCheckTime", ""),
        },
        webdav: WebDavConfig {
            url: parse_str(&m, "webdav_url", ""),
            username: parse_str(&m, "webdav_username", ""),
            password: parse_str(&m, "webdav_password", ""),
            config_path: parse_str(&m, "webdav_configPath", ""),
            stats_path: parse_str(&m, "webdav_statsPath", ""),
        },
    })
}
