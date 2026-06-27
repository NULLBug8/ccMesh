use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamDiagnostic {
    pub reason: String,
    pub action: String,
    pub evidence: String,
}

impl UpstreamDiagnostic {
    pub fn format_for_endpoint_test(&self, status: u16, url: &str, model: &str) -> String {
        format!(
            "测试失败：{}。处理方式：{}。HTTP {status}，测试 URL: {url}，模型: {model}。依据：{}",
            self.reason, self.action, self.evidence
        )
    }

    pub fn format_for_proxy(&self, endpoint: &str, status: u16, model: Option<&str>) -> String {
        let model = model.unwrap_or("-");
        format!(
            "{endpoint}: {}。处理方式：{}。HTTP {status}，模型: {model}。依据：{}",
            self.reason, self.action, self.evidence
        )
    }
}

pub fn diagnose_upstream_error(status: u16, body: &str) -> UpstreamDiagnostic {
    let evidence = extract_evidence(body);
    let haystack = evidence.to_ascii_lowercase();

    if contains_any(
        &haystack,
        &[
            "codex_access_restricted",
            "最新版的codex客户端",
            "最新版 codex",
            "latest codex",
            "codex client",
            "codex cli",
        ],
    ) {
        return UpstreamDiagnostic {
            reason: "上游限制 Codex 客户端版本或客户端标识".into(),
            action: "升级 Codex CLI/客户端；如果这里是通过 ccMesh 测试，请到设置里检查 OpenAI/Codex User-Agent 伪装值，改成该站点要求的最新版 Codex UA，并保留 originator: codex_cli_rs；改完后重新测试该端点".into(),
            evidence,
        };
    }

    if haystack.contains("tool_choice")
        && haystack.contains("tools")
        && contains_any(
            &haystack,
            &[
                "must be set",
                "must be provided",
                "is required",
                "required when",
            ],
        )
    {
        return UpstreamDiagnostic {
            reason: "请求包含 tool_choice，但没有同时提供 tools".into(),
            action: "去掉 tool_choice，或在同一个请求体里提供 tools；如果这是端点连通性测试触发的，说明测试探针不应发送 tool_choice，应升级到修复后的版本再测".into(),
            evidence,
        };
    }

    if contains_any(
        &haystack,
        &[
            "invalid api key",
            "incorrect api key",
            "api key is invalid",
            "invalid token",
            "unauthorized",
            "authentication",
            "no auth",
            "missing api key",
        ],
    ) {
        return UpstreamDiagnostic {
            reason: "API Key 无效或鉴权头不被上游接受".into(),
            action: "重新填写该端点 API Key；确认端点类型对应的鉴权方式正确；如果站点要求 Bearer，就用 openai/codex 类型，如果要求 x-api-key，就用 claude 类型".into(),
            evidence,
        };
    }

    if contains_any(
        &haystack,
        &[
            "insufficient_quota",
            "quota exceeded",
            "balance",
            "credit",
            "billing",
            "额度",
            "余额",
            "欠费",
        ],
    ) {
        return UpstreamDiagnostic {
            reason: "上游额度不足或账号欠费".into(),
            action: "到余额页查询该站点余额/套餐限额；补充额度后重新测试；如果只有某个模型失败，检查该模型是否单独计费或被套餐排除".into(),
            evidence,
        };
    }

    if contains_any(
        &haystack,
        &[
            "rate limit",
            "too many requests",
            "temporarily blocked",
            "risk",
            "waf",
            "forbidden by",
            "frequency",
            "风控",
            "频率",
            "限流",
        ],
    ) {
        return UpstreamDiagnostic {
            reason: "上游限流或临时风控拦截".into(),
            action: "降低并发/请求频率，稍后重试；如果同站点同模型偶发成功，不要改 Key，优先检查站点限流规则和当前账号限额".into(),
            evidence,
        };
    }

    if contains_any(
        &haystack,
        &[
            "model_not_found",
            "model not found",
            "model does not exist",
            "unknown model",
            "invalid model",
            "no such model",
            "模型不存在",
            "不支持该模型",
        ],
    ) {
        return UpstreamDiagnostic {
            reason: "上游不认识当前出站模型名".into(),
            action: "在端点模型列表里选择该站点真实存在的模型；如果客户端要用别名，在模型映射里把入站模型映射到上游真实模型；保存后重新测试这个模型".into(),
            evidence,
        };
    }

    if contains_any(
        &haystack,
        &[
            "context_length_exceeded",
            "maximum context",
            "context length",
            "token limit",
            "too many tokens",
            "request too large",
            "payload too large",
            "body too large",
            "length limit",
            "上下文",
            "请求体过大",
        ],
    ) {
        return UpstreamDiagnostic {
            reason: "请求体或上下文超过上游限制".into(),
            action: "减少输入内容/工具返回内容/历史消息；降低 max_tokens/max_output_tokens；如果是本地 413，升级后本地限制已放宽到 64MB，但上游仍可能有自己的限制".into(),
            evidence,
        };
    }

    if contains_any(
        &haystack,
        &[
            "unsupported",
            "not supported",
            "input must be a list",
            "messages is required",
            "invalid request body",
            "response.completed",
            "stream disconnected",
        ],
    ) {
        return UpstreamDiagnostic {
            reason: "端点类型或请求格式与上游接口不匹配".into(),
            action: "确认端点类型：原生 Responses 站点用 codex；只支持 /v1/chat/completions 的站点用 openai；如果测试通过但实际调用失败，优先检查真实调用是否包含工具、图片、超长 input 或该站点不支持 Responses 流式完整事件".into(),
            evidence,
        };
    }

    if status == 401 {
        return UpstreamDiagnostic {
            reason: "上游明确拒绝鉴权".into(),
            action: "重新填写 API Key，并确认端点类型和鉴权方式匹配".into(),
            evidence,
        };
    }
    if status == 403 {
        return UpstreamDiagnostic {
            reason: "上游拒绝访问，但返回体没有说明具体原因".into(),
            action: "先查看返回体原文；如果同站点其他模型可用，检查该模型权限/套餐；如果偶发成功，按临时风控或频率限制处理，不要直接判定 Key 错".into(),
            evidence,
        };
    }
    if status == 404 {
        return UpstreamDiagnostic {
            reason: "接口路径不存在".into(),
            action: "检查 API 地址是否只填写站点根域名；确认端点类型会拼接正确路径，例如 codex -> /v1/responses，openai -> /v1/chat/completions".into(),
            evidence,
        };
    }
    if status == 429 {
        return UpstreamDiagnostic {
            reason: "上游限流或额度不足".into(),
            action: "降低请求频率，检查余额和套餐限额，稍后重试".into(),
            evidence,
        };
    }
    if status >= 500 {
        return UpstreamDiagnostic {
            reason: "上游服务异常或站点当前不稳定".into(),
            action: "稍后重试；如果频繁出现，把该端点暂时禁用或降低排序，避免路由轮询选中它".into(),
            evidence,
        };
    }

    UpstreamDiagnostic {
        reason: format!("上游返回 HTTP {status}"),
        action: "查看返回体原文，根据上游错误码调整 API Key、模型映射、端点类型或请求内容".into(),
        evidence,
    }
}

fn extract_evidence(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return "上游没有返回错误体".into();
    }
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        let mut parts = Vec::new();
        collect_json_strings(&value, &mut parts);
        if !parts.is_empty() {
            return truncate(&parts.join(" | "), 300);
        }
    }
    truncate(trimmed, 300)
}

fn collect_json_strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) => {
            if !s.trim().is_empty() {
                out.push(s.trim().to_string());
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_json_strings(item, out);
            }
        }
        Value::Object(map) => {
            for key in [
                "code", "type", "message", "detail", "error", "error_msg", "error_message",
                "reason",
            ] {
                if let Some(value) = map.get(key) {
                    collect_json_strings(value, out);
                }
            }
            for (key, value) in map {
                if !matches!(
                    key.as_str(),
                    "code"
                        | "type"
                        | "message"
                        | "detail"
                        | "error"
                        | "error_msg"
                        | "error_message"
                        | "reason"
                ) {
                    collect_json_strings(value, out);
                }
            }
        }
        _ => {}
    }
}

fn contains_any(text: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| text.contains(needle))
}

fn truncate(text: &str, max_chars: usize) -> String {
    let mut out = text.chars().take(max_chars).collect::<String>();
    if text.chars().count() > max_chars {
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::diagnose_upstream_error;

    #[test]
    fn diagnoses_model_mapping_issue_from_json_error() {
        let diag = diagnose_upstream_error(
            400,
            r#"{"error":{"code":"model_not_found","message":"model gpt-5.5 does not exist"}}"#,
        );

        assert!(diag.reason.contains("出站模型名"));
        assert!(diag.action.contains("模型映射"));
    }

    #[test]
    fn diagnoses_permission_or_transient_403_from_body() {
        let diag = diagnose_upstream_error(403, r#"{"message":"temporarily blocked by WAF"}"#);

        assert!(diag.reason.contains("限流") || diag.reason.contains("风控"));
        assert!(diag.action.contains("不要直接判定 Key 错") || diag.action.contains("降低"));
    }

    #[test]
    fn diagnoses_codex_client_version_restriction_before_generic_403() {
        let diag = diagnose_upstream_error(
            403,
            r#"{"error":{"message":"请使用最新版的codex客户端或codex cli调用","type":"invalid_request_error","code":"codex_access_restricted"}}"#,
        );

        assert!(diag.reason.contains("Codex 客户端版本"));
        assert!(diag.action.contains("User-Agent"));
        assert!(diag.evidence.contains("codex_access_restricted"));
    }

    #[test]
    fn diagnoses_tool_choice_without_tools_before_generic_format_error() {
        let diag = diagnose_upstream_error(
            200,
            r#"data: {"code":"InvalidParameter","message":"<400> InternalError.Algo.InvalidParameter: When using `tool_choice`, `tools` must be set."}"#,
        );

        assert!(diag.reason.contains("tool_choice"));
        assert!(diag.reason.contains("tools"));
        assert!(diag.action.contains("去掉 tool_choice"));
        assert!(diag.evidence.contains("tools"));
    }

    #[test]
    fn diagnoses_request_format_mismatch() {
        let diag = diagnose_upstream_error(400, r#"{"detail":"Input must be a list"}"#);

        assert!(diag.reason.contains("请求格式"));
        assert!(diag.action.contains("端点类型"));
    }

    #[test]
    fn formats_endpoint_message_with_action_and_evidence() {
        let diag = diagnose_upstream_error(429, r#"{"error":"quota exceeded"}"#);
        let message =
            diag.format_for_endpoint_test(429, "https://example.com/v1/responses", "gpt-5.5");

        assert!(message.contains("处理方式"));
        assert!(message.contains("依据"));
    }
}
