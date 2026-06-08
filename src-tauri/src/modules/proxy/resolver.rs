use std::collections::HashMap;

use crate::models::endpoint::Endpoint;

/// 端点指定头部（项目决策：`X-CCmomo-Endpoint`），及兼容别名。
pub const ENDPOINT_HEADER: &str = "x-ccmomo-endpoint";
pub const ENDPOINT_HEADER_ALT: &str = "x-endpoint-name";
pub const ENDPOINT_QUERY: &str = "endpoint";
pub const ENDPOINT_QUERY_ALT: &str = "ep";

/// 解析结果：命中的指定端点（None = 走轮换）+ 可选模型覆盖 + 未找到错误。
#[derive(Debug, Default, Clone)]
pub struct Resolution {
    pub endpoint: Option<Endpoint>,
    /// 每请求模型覆盖（`@端点/模型` 语法解析所得，已有单测）；转发侧应用待接入，暂保留。
    #[allow(dead_code)]
    pub model_override: Option<String>,
    /// 指定了端点名但未找到/未启用 → 错误信息（调用方应返回 400）。
    pub not_found: Option<String>,
}

impl Resolution {
    /// 是否使用了「指定端点」（命中后不轮换）。
    pub fn use_specific(&self) -> bool {
        self.endpoint.is_some()
    }
}

/// 按优先级解析端点：① HTTP 头部 ② 模型名 `@端点名/模型名` ③ 查询参数。
/// `headers` 与 `query` 的键应为小写。`enabled` 为当前启用端点列表（大小写不敏感匹配）。
pub fn resolve_endpoint(
    headers: &HashMap<String, String>,
    model: Option<&str>,
    query: &HashMap<String, String>,
    enabled: &[Endpoint],
) -> Resolution {
    // ① 头部
    if let Some(name) = headers
        .get(ENDPOINT_HEADER)
        .or_else(|| headers.get(ENDPOINT_HEADER_ALT))
    {
        let name = name.trim();
        if !name.is_empty() {
            return by_name(name, None, enabled);
        }
    }

    // ② 模型名 @端点/模型
    if let Some(m) = model {
        if let Some(stripped) = m.strip_prefix('@') {
            let (ep_name, model_override) = match stripped.split_once('/') {
                Some((e, mo)) => (e.trim(), Some(mo.to_string())),
                None => (stripped.trim(), None),
            };
            if !ep_name.is_empty() {
                return by_name(ep_name, model_override, enabled);
            }
        }
    }

    // ③ 查询参数
    if let Some(name) = query
        .get(ENDPOINT_QUERY)
        .or_else(|| query.get(ENDPOINT_QUERY_ALT))
    {
        let name = name.trim();
        if !name.is_empty() {
            return by_name(name, None, enabled);
        }
    }

    Resolution::default()
}

fn by_name(name: &str, model_override: Option<String>, enabled: &[Endpoint]) -> Resolution {
    match enabled
        .iter()
        .find(|e| e.name.trim().eq_ignore_ascii_case(name))
    {
        Some(e) => Resolution {
            endpoint: Some(e.clone()),
            model_override,
            not_found: None,
        },
        None => Resolution {
            endpoint: None,
            model_override: None,
            not_found: Some(format!("指定的端点 '{name}' 不存在或未启用")),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ep(name: &str) -> Endpoint {
        Endpoint {
            id: 1,
            name: name.to_string(),
            api_url: "https://x".into(),
            api_key: "".into(),
            auth_mode: "api_key".into(),
            enabled: true,
            use_proxy: false,
            transformer: "claude".into(),
            model: "".into(),
            models: Vec::new(),
            remark: "".into(),
            sort_order: 0,
            test_status: "unknown".into(),
            created_at: "".into(),
            updated_at: "".into(),
        }
    }

    fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn resolves_by_header() {
        let eps = vec![ep("alpha"), ep("beta")];
        let r = resolve_endpoint(&map(&[("x-ccmomo-endpoint", "beta")]), None, &HashMap::new(), &eps);
        assert_eq!(r.endpoint.unwrap().name, "beta");
    }

    #[test]
    fn resolves_by_model_with_override() {
        let eps = vec![ep("alpha")];
        let r = resolve_endpoint(&HashMap::new(), Some("@alpha/gpt-4o"), &HashMap::new(), &eps);
        assert_eq!(r.endpoint.unwrap().name, "alpha");
        assert_eq!(r.model_override.as_deref(), Some("gpt-4o"));
    }

    #[test]
    fn resolves_by_model_without_slash() {
        let eps = vec![ep("alpha")];
        let r = resolve_endpoint(&HashMap::new(), Some("@alpha"), &HashMap::new(), &eps);
        assert_eq!(r.endpoint.unwrap().name, "alpha");
        assert!(r.model_override.is_none());
    }

    #[test]
    fn resolves_by_query() {
        let eps = vec![ep("alpha")];
        let r = resolve_endpoint(&HashMap::new(), None, &map(&[("ep", "alpha")]), &eps);
        assert_eq!(r.endpoint.unwrap().name, "alpha");
    }

    #[test]
    fn header_takes_priority_over_model() {
        let eps = vec![ep("alpha"), ep("beta")];
        let r = resolve_endpoint(&map(&[("x-ccmomo-endpoint", "alpha")]), Some("@beta/m"), &HashMap::new(), &eps);
        assert_eq!(r.endpoint.unwrap().name, "alpha");
    }

    #[test]
    fn unknown_name_yields_not_found_and_no_rotation_endpoint() {
        let eps = vec![ep("alpha")];
        let r = resolve_endpoint(&map(&[("x-ccmomo-endpoint", "ghost")]), None, &HashMap::new(), &eps);
        assert!(r.endpoint.is_none());
        assert!(r.not_found.is_some());
    }

    #[test]
    fn no_hint_falls_back_to_rotation() {
        let eps = vec![ep("alpha")];
        let r = resolve_endpoint(&HashMap::new(), Some("gpt-4o"), &HashMap::new(), &eps);
        assert!(r.endpoint.is_none());
        assert!(r.not_found.is_none());
        assert!(!r.use_specific());
    }
}
