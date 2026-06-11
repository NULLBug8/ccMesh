use serde_json::Value;

/// 上游 API 格式，决定客户端（Claude Messages）请求是否需要转换。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamFormat {
    Claude,
    OpenAiChat,
    /// 上游原生 OpenAI Responses API（codex 端点）。请求/响应转换与透传由
    /// [`super::responses_chat`] 处理，不走 [`Transformer`] trait（入站是 Responses 而非 Claude）。
    OpenAiResponses,
}

impl UpstreamFormat {
    /// 由端点 `transformer` 字段推断上游格式。
    /// codex / openai_responses → Responses API；gemini 等其余未知值仍按 Claude 直通处理。
    pub fn from_transformer_name(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "openai" | "openai_chat" | "openai-chat" | "openai2" => UpstreamFormat::OpenAiChat,
            "codex" | "openai_responses" | "openai-responses" => UpstreamFormat::OpenAiResponses,
            _ => UpstreamFormat::Claude,
        }
    }
}

/// 格式转换器：客户端固定为 Claude Messages 格式，按上游格式双向转换。
pub trait Transformer: Send + Sync {
    /// Claude 请求体 → 上游请求体。`endpoint_model` 为端点配置的模型名（覆盖请求 model）。
    fn transform_request(
        &self,
        claude_request: &Value,
        endpoint_model: Option<&str>,
    ) -> crate::error::AppResult<Value>;
}

/// 直通转换器（上游本身即 Claude 格式）。
pub struct IdentityTransformer;

impl Transformer for IdentityTransformer {
    fn transform_request(
        &self,
        req: &Value,
        _endpoint_model: Option<&str>,
    ) -> crate::error::AppResult<Value> {
        Ok(req.clone())
    }
}

/// 按上游格式返回对应转换器。
pub fn get_transformer(format: UpstreamFormat) -> Box<dyn Transformer> {
    match format {
        UpstreamFormat::Claude => Box::new(IdentityTransformer),
        UpstreamFormat::OpenAiChat => Box::new(super::claude_openai::ClaudeOpenAiTransformer),
        // Responses（codex）请求转换不走此 trait（入站为 Responses 而非 Claude），由 responses_chat 处理；
        // 此处返回 Identity 仅为穷尽 match。
        UpstreamFormat::OpenAiResponses => Box::new(IdentityTransformer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_maps_to_responses() {
        assert_eq!(
            UpstreamFormat::from_transformer_name("codex"),
            UpstreamFormat::OpenAiResponses
        );
        assert_eq!(
            UpstreamFormat::from_transformer_name("openai_responses"),
            UpstreamFormat::OpenAiResponses
        );
    }

    #[test]
    fn openai_aliases_map_to_chat() {
        for n in ["openai", "openai_chat", "openai-chat", "openai2"] {
            assert_eq!(
                UpstreamFormat::from_transformer_name(n),
                UpstreamFormat::OpenAiChat
            );
        }
    }

    #[test]
    fn unknown_maps_to_claude() {
        assert_eq!(
            UpstreamFormat::from_transformer_name("gemini"),
            UpstreamFormat::Claude
        );
    }
}
