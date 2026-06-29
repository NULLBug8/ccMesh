//! 可用性探测时模拟真实官方客户端的 User-Agent。
//!
//! 网关主动发起的探测请求（端点连通性测试、模型候选拉取）默认没有客户端 UA。
//! 部分上游会对「无 UA / 非官方客户端」拦截（如 Cloudflare 对空 UA 返回 403），
//! 故按上游格式模拟对应官方 CLI 的 UA：
//! - Claude 端点 → Claude CLI：`claude-cli/<version> (external, sdk-cli)`。
//! - OpenAI 端点 → Codex CLI：`codex_cli_rs/<version> (<OS>; <arch>) vscode/<version>`，并附 `originator: codex_cli_rs` 头。

/// 模拟 Claude CLI 的 UA（版本号随发布更新，此处先固定一个近期版本）。
pub const CLAUDE_PROBE_UA: &str = "claude-cli/2.1.185 (external, sdk-cli)";

/// Codex CLI 的 originator 头值，后端据此识别真实客户端。
pub const CODEX_ORIGINATOR: &str = "codex_cli_rs";

pub fn is_invalid_openai_codex_ua(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower.is_empty()
        || lower.contains("local server discovery")
        || lower.contains("server discovery")
}

pub fn usable_openai_codex_ua(value: &str) -> Option<&str> {
    let value = value.trim();
    if is_invalid_openai_codex_ua(value) {
        None
    } else {
        Some(value)
    }
}

/// 模拟 Codex CLI 的 UA：`codex_cli_rs/<version> (<OS>; <arch>) vscode/<version>`，OS/arch 取运行环境。
pub fn codex_probe_ua() -> String {
    format!(
        "codex_cli_rs/0.114.0 ({}; {}) vscode/1.111.0",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}
