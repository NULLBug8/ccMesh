use serde_json::{json, Value};

use crate::models::endpoint::Endpoint;
use crate::modules::models_probe::{self, ProbeAuth};
use crate::modules::transform::transformer::UpstreamFormat;

fn default_model(ep: &Endpoint) -> &str {
    if ep.model.is_empty() {
        UpstreamFormat::from_transformer_name(&ep.transformer).default_model()
    } else {
        ep.model.as_str()
    }
}

pub fn model_info(id: &str, endpoint_name: &str) -> Value {
    json!({
        "id": id,
        "object": "model",
        "created": 1_735_689_600,
        "owned_by": endpoint_name,
        "endpoint_id": endpoint_name
    })
}

/// 拉取指定上游的模型 id 列表：单 URL 单鉴权一次性尝试（get_models 聚合路径用；
/// 表单刷新走 [`models_probe::probe_models`] 聚合策略）。失败返回空。
async fn fetch_model_ids(
    client: &reqwest::Client,
    api_url: &str,
    api_key: &str,
    transformer: &str,
    openai_ua: Option<&str>,
    claude_ua: Option<&str>,
) -> Vec<String> {
    let url = models_probe::models_url_from_base(api_url);
    let auth = ProbeAuth::primary_for(transformer);
    models_probe::request_model_ids_with_ua(client, &url, api_key, auth, openai_ua, claude_ua).await
}

/// 拉取单个端点的模型列表（OpenAI 走 `/v1/models`；Claude 或失败回落默认模型）。
pub async fn fetch_models_with_ua(
    client: &reqwest::Client,
    ep: &Endpoint,
    openai_ua: Option<&str>,
    claude_ua: Option<&str>,
) -> Vec<Value> {
    let ids = fetch_model_ids(
        client,
        &ep.api_url,
        &ep.api_key,
        &ep.transformer,
        openai_ua,
        claude_ua,
    )
    .await;
    if ids.is_empty() {
        vec![model_info(default_model(ep), &ep.name)]
    } else {
        ids.iter().map(|id| model_info(id, &ep.name)).collect()
    }
}
