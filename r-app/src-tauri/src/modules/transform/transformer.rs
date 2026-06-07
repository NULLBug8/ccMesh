use serde_json::Value;

/// 上游 API 格式，决定客户端（Claude Messages）请求是否需要转换。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamFormat {
    Claude,
    OpenAiChat,
}

impl UpstreamFormat {
    /// 由端点 `transformer` 字段推断上游格式。
    /// gemini / openai_responses / codex 等在本项目 Out of Scope，按 Claude 直通处理。
    pub fn from_transformer_name(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "openai" | "openai_chat" | "openai-chat" | "openai2" => UpstreamFormat::OpenAiChat,
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
        UpstreamFormat::OpenAiChat => {
            Box::new(super::claude_openai::ClaudeOpenAiTransformer)
        }
    }
}
