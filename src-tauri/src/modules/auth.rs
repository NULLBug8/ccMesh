use axum::body::Body;
use axum::extract::Request;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{Html, IntoResponse, Response};
use axum::Json;
use serde::Deserialize;
use serde_json::json;

pub const ADMIN_TOKEN_ENV: &str = "CCMESH_ADMIN_TOKEN";
pub const SESSION_COOKIE: &str = "ccmesh_admin_session";

fn configured_token() -> Option<String> {
    std::env::var(ADMIN_TOKEN_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (left, right) in a.iter().zip(b.iter()) {
        diff |= left ^ right;
    }
    diff == 0
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn header_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("x-ccmesh-admin-token")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn cookie_token(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in cookie.split(';') {
        if let Some((name, value)) = part.trim().split_once('=') {
            if name == SESSION_COOKIE {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

pub fn request_is_authorized(headers: &HeaderMap) -> bool {
    let Some(expected) = configured_token() else {
        return false;
    };

    bearer_token(headers)
        .or_else(|| header_token(headers))
        .map(|token| constant_time_eq(token, &expected))
        .unwrap_or(false)
        || cookie_token(headers)
            .map(|token| constant_time_eq(&token, &expected))
            .unwrap_or(false)
}

fn accepts_html(headers: &HeaderMap, path: &str) -> bool {
    path == "/"
        || headers
            .get(header::ACCEPT)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.contains("text/html"))
            .unwrap_or(false)
}

pub async fn require_admin_auth(headers: HeaderMap, request: Request, next: Next) -> Response {
    if request_is_authorized(&headers) {
        return next.run(request).await;
    }

    if configured_token().is_none() {
        return auth_not_configured_response();
    }

    if accepts_html(&headers, request.uri().path()) {
        return login_page_response();
    }

    unauthorized_response()
}

#[derive(Deserialize)]
pub struct LoginRequest {
    token: String,
}

pub async fn login_page() -> Response {
    if configured_token().is_none() {
        return auth_not_configured_response();
    }
    login_page_response()
}

pub async fn login(Json(body): Json<LoginRequest>) -> Response {
    let Some(expected) = configured_token() else {
        return auth_not_configured_response();
    };
    if !constant_time_eq(body.token.trim(), &expected) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "访问令牌错误" })),
        )
            .into_response();
    }

    let cookie = format!(
        "{SESSION_COOKIE}={}; Path=/; HttpOnly; SameSite=Lax; Max-Age=2592000",
        body.token.trim()
    );
    let mut response = Json(json!({ "ok": true })).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).unwrap_or_else(|_| HeaderValue::from_static("")),
    );
    response
}

pub async fn logout() -> Response {
    let mut response = (StatusCode::FOUND, "").into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static(
            "ccmesh_admin_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0",
        ),
    );
    response
        .headers_mut()
        .insert(header::LOCATION, HeaderValue::from_static("/"));
    response
}

fn unauthorized_response() -> Response {
    (StatusCode::UNAUTHORIZED, Body::from("Unauthorized")).into_response()
}

fn auth_not_configured_response() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Html(
            r#"<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><title>ccMesh locked</title><style>body{margin:0;min-height:100vh;display:grid;place-items:center;background:#09090b;color:#fafafa;font-family:system-ui}.card{max-width:520px;padding:28px;border:1px solid #27272a;border-radius:18px;background:#18181b}code{color:#facc15}</style></head><body><main class="card"><h1>ccMesh 已锁定</h1><p>服务器未配置访问令牌。为避免公网泄露任何信息，控制台和代理接口已拒绝访问。</p><p>请设置环境变量：<code>CCMESH_ADMIN_TOKEN</code></p></main></body></html>"#,
        ),
    )
        .into_response()
}

fn login_page_response() -> Response {
    Html(
        r#"<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>ccMesh 登录</title>
  <style>
    *{box-sizing:border-box}body{margin:0;min-height:100vh;display:grid;place-items:center;background:radial-gradient(circle at 20% 10%,#1e3a8a55,transparent 32%),#09090b;color:#fafafa;font-family:Inter,system-ui,-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}.card{width:min(420px,calc(100vw - 32px));padding:30px;border:1px solid #27272a;border-radius:22px;background:#18181bcc;box-shadow:0 24px 80px #0008;backdrop-filter:blur(18px)}h1{margin:0 0 8px;font-size:24px}p{margin:0 0 22px;color:#a1a1aa;line-height:1.6}label{display:block;margin-bottom:8px;color:#d4d4d8;font-size:14px}input{width:100%;height:44px;border:1px solid #3f3f46;border-radius:12px;background:#09090b;color:#fafafa;padding:0 13px;outline:none}input:focus{border-color:#60a5fa;box-shadow:0 0 0 3px #2563eb44}button{width:100%;height:44px;margin-top:14px;border:0;border-radius:12px;background:#2563eb;color:white;font-weight:700;cursor:pointer}button:disabled{opacity:.55;cursor:not-allowed}.error{min-height:20px;margin-top:12px;color:#f87171;font-size:13px}
  </style>
</head>
<body>
  <main class="card">
    <h1>ccMesh 访问认证</h1>
    <p>此服务器已启用访问保护。请输入管理员访问令牌。</p>
    <form id="login">
      <label for="token">访问令牌</label>
      <input id="token" name="token" type="password" autocomplete="current-password" autofocus />
      <button id="submit" type="submit">登录</button>
      <div id="error" class="error"></div>
    </form>
  </main>
  <script>
    const form = document.getElementById('login');
    const input = document.getElementById('token');
    const button = document.getElementById('submit');
    const error = document.getElementById('error');
    form.addEventListener('submit', async (event) => {
      event.preventDefault();
      error.textContent = '';
      button.disabled = true;
      try {
        const response = await fetch('/__auth/login', {
          method: 'POST',
          headers: {'content-type':'application/json'},
          body: JSON.stringify({token: input.value})
        });
        if (!response.ok) {
          const payload = await response.json().catch(() => null);
          throw new Error(payload?.error || '访问令牌错误');
        }
        location.replace('/');
      } catch (err) {
        error.textContent = err?.message || '登录失败';
      } finally {
        button.disabled = false;
      }
    });
  </script>
</body>
</html>"#,
    )
    .into_response()
}

#[cfg(test)]
mod tests {
    use axum::http::{header::AUTHORIZATION, HeaderMap, HeaderValue};
    use std::sync::{Mutex, OnceLock};

    use super::request_is_authorized;

    fn with_admin_token<T>(token: Option<&str>, test: impl FnOnce() -> T) -> T {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous = std::env::var("CCMESH_ADMIN_TOKEN").ok();
        match token {
            Some(value) => std::env::set_var("CCMESH_ADMIN_TOKEN", value),
            None => std::env::remove_var("CCMESH_ADMIN_TOKEN"),
        }
        let result = test();
        match previous {
            Some(value) => std::env::set_var("CCMESH_ADMIN_TOKEN", value),
            None => std::env::remove_var("CCMESH_ADMIN_TOKEN"),
        }
        result
    }

    #[test]
    fn bearer_token_authorizes_request_when_configured() {
        with_admin_token(Some("secret-token"), || {
            let mut headers = HeaderMap::new();
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_static("Bearer secret-token"),
            );

            assert!(request_is_authorized(&headers));
        });
    }

    #[test]
    fn missing_or_wrong_token_is_rejected_when_configured() {
        with_admin_token(Some("secret-token"), || {
            assert!(!request_is_authorized(&HeaderMap::new()));

            let mut headers = HeaderMap::new();
            headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer wrong"));
            assert!(!request_is_authorized(&headers));
        });
    }

    #[test]
    fn session_cookie_authorizes_browser_requests() {
        with_admin_token(Some("secret-token"), || {
            let mut headers = HeaderMap::new();
            headers.insert(
                axum::http::header::COOKIE,
                HeaderValue::from_static("theme=dark; ccmesh_admin_session=secret-token"),
            );

            assert!(request_is_authorized(&headers));
        });
    }

    #[test]
    fn unset_admin_token_rejects_everything_by_default() {
        with_admin_token(None, || {
            let mut headers = HeaderMap::new();
            headers.insert(
                AUTHORIZATION,
                HeaderValue::from_static("Bearer any-token"),
            );

            assert!(!request_is_authorized(&headers));
        });
    }
}
