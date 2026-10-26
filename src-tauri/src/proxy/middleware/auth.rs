// API Key 认证中间件
use axum::{
    extract::Request,
    extract::State,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use std::borrow::Cow;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::proxy::{ProxyAuthMode, ProxySecurityConfig};

/// Trim a header value and drop it when nothing is left.
fn non_empty_trimmed(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

/// Extract the client credential from a request.
///
/// Order of precedence:
/// 1. `Authorization` header (`Bearer <t>`, `bearer <t>`, or a bare token)
/// 2. `x-api-key` header
/// 3. `x-goog-api-key` header
/// 4. URL query `?key=<t>` (Google Gemini REST convention), only when `allow_query` is true
///
/// `allow_query` must be false for management (admin) endpoints: a credential in the URL
/// ends up in browser history, proxies and access logs, so the admin password is only
/// accepted from headers.
pub fn extract_api_key<B>(request: &Request<B>, allow_query: bool) -> Option<Cow<'_, str>> {
    let headers = request.headers();

    if let Some(auth_val) = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
    {
        let trimmed = auth_val.trim();
        let token = if trimmed.len() >= 7
            && trimmed.is_char_boundary(7)
            && trimmed[..7].eq_ignore_ascii_case("bearer ")
        {
            trimmed[7..].trim()
        } else if trimmed.eq_ignore_ascii_case("bearer") {
            ""
        } else {
            trimmed
        };
        if !token.is_empty() {
            return Some(Cow::Borrowed(token));
        }
    }

    for name in ["x-api-key", "x-goog-api-key"] {
        if let Some(token) = headers
            .get(name)
            .and_then(|h| h.to_str().ok())
            .and_then(non_empty_trimmed)
        {
            return Some(Cow::Borrowed(token));
        }
    }

    if allow_query {
        if let Some(query) = request.uri().query() {
            for (name, value) in url::form_urlencoded::parse(query.as_bytes()) {
                if name != "key" {
                    continue;
                }
                match value {
                    Cow::Borrowed(v) => {
                        if let Some(token) = non_empty_trimmed(v) {
                            return Some(Cow::Borrowed(token));
                        }
                    }
                    Cow::Owned(v) => {
                        if let Some(token) = non_empty_trimmed(&v) {
                            return Some(Cow::Owned(token.to_string()));
                        }
                    }
                }
            }
        }
    }

    None
}

/// API Key 认证中间件 (代理接口使用，遵循 auth_mode)
pub async fn auth_middleware(
    state: State<Arc<RwLock<ProxySecurityConfig>>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    auth_middleware_internal(state, request, next, false).await
}

/// 管理接口认证中间件 (管理接口使用，强制严格鉴权)
pub async fn admin_auth_middleware(
    state: State<Arc<RwLock<ProxySecurityConfig>>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    auth_middleware_internal(state, request, next, true).await
}

/// 内部认证逻辑
async fn auth_middleware_internal(
    State(security): State<Arc<RwLock<ProxySecurityConfig>>>,
    request: Request,
    next: Next,
    force_strict: bool,
) -> Result<Response, StatusCode> {
    let method = request.method().clone();
    let path = request.uri().path().to_string();

    // 过滤心跳和健康检查请求,避免日志噪音
    let is_health_check = path == "/healthz" || path == "/api/health" || path == "/health";
    let is_internal_endpoint = crate::proxy::middleware::internal::is_internal_path(&path);
    if !path.contains("event_logging") && !is_health_check {
        tracing::info!("Request: {} {}", method, path);
    } else {
        tracing::trace!("Heartbeat/Health: {} {}", method, path);
    }

    // Internal endpoints require the outer middleware's private validation marker
    // before any auth or OPTIONS bypass.
    if is_internal_endpoint {
        crate::proxy::middleware::internal::require_validated_internal_request(&request)?;
    }

    // Allow CORS preflight regardless of auth policy.
    if method == axum::http::Method::OPTIONS {
        return Ok(next.run(request).await);
    }

    let security = security.read().await.clone();
    let effective_mode = security.effective_auth_mode();

    // 权限检查逻辑
    if !force_strict {
        // AI 代理接口 (v1/chat/completions 等)
        if matches!(effective_mode, ProxyAuthMode::Off) {
            // [FIX] 即使 auth_mode=Off，也需要尝试识别 User Token 以记录使用情况
            // 先检查是否携带了 User Token
            let api_key = extract_api_key(&request, true);

            if let Some(token) = api_key.as_deref() {
                // 尝试验证是否为 User Token（不阻止请求，只记录）
                if let Ok(Some(user_token)) =
                    crate::modules::user_token_db::get_token_by_value(token)
                {
                    let identity = UserTokenIdentity {
                        token_id: user_token.id,
                        token: user_token.token,
                        username: user_token.username,
                    };
                    // 注入 identity 到请求
                    let (mut parts, body) = request.into_parts();
                    parts.extensions.insert(identity);
                    let request = Request::from_parts(parts, body);
                    return Ok(next.run(request).await);
                }
            }

            return Ok(next.run(request).await);
        }

        if matches!(effective_mode, ProxyAuthMode::AllExceptHealth) && is_health_check {
            return Ok(next.run(request).await);
        }

        // 内部端点 (/internal/*) 豁免鉴权 - 用于 warmup 等内部功能
        if is_internal_endpoint {
            tracing::debug!("Internal endpoint bypassed auth: {}", path);
            return Ok(next.run(request).await);
        }
    } else {
        // Management endpoints always require admin auth; only health checks stay public.
        if is_health_check {
            return Ok(next.run(request).await);
        }
    }

    // 从 header 提取 API key；仅代理接口额外接受 URL query `?key=`（Gemini REST 规范），
    // 管理接口永远不从 URL 取凭证，避免管理密码进入 URL / 日志。
    let api_key = extract_api_key(&request, !force_strict);

    if security.api_key.is_empty()
        && (security.admin_password.is_none()
            || security.admin_password.as_ref().unwrap().is_empty())
    {
        if force_strict {
            tracing::error!("Admin auth is required but both api_key and admin_password are empty; denying request");
            return Err(StatusCode::UNAUTHORIZED);
        }
        tracing::error!("Proxy auth is enabled but api_key is empty; denying request");
        return Err(StatusCode::UNAUTHORIZED);
    }

    // 认证逻辑
    let authorized = if force_strict {
        // 管理接口：优先使用独立的 admin_password，如果没有则回退使用 api_key
        match &security.admin_password {
            Some(pwd) if !pwd.is_empty() => api_key.as_deref().map(|k| k == pwd).unwrap_or(false),
            _ => {
                // 回退使用 api_key
                api_key
                    .as_deref()
                    .map(|k| k == security.api_key)
                    .unwrap_or(false)
            }
        }
    } else {
        // AI 代理接口：仅允许使用 api_key
        api_key
            .as_deref()
            .map(|k| k == security.api_key)
            .unwrap_or(false)
    };

    if authorized {
        Ok(next.run(request).await)
    } else if !force_strict && api_key.is_some() {
        // 尝试验证 UserToken
        let token = api_key.as_deref().unwrap();

        // 提取 IP (复用 ip_filter 规范化逻辑，支持 IPv4/IPv6 及 ConnectInfo)
        let client_ip = crate::proxy::middleware::ip_filter::extract_client_ip(&request)
            .unwrap_or_else(|| "127.0.0.1".to_string()); // Default fallback

        // 验证 Token
        match crate::modules::user_token_db::validate_token(token, &client_ip) {
            Ok((true, _)) => {
                // Token 有效，查询信息以便传递
                if let Ok(Some(user_token)) =
                    crate::modules::user_token_db::get_token_by_value(token)
                {
                    let identity = UserTokenIdentity {
                        token_id: user_token.id,
                        token: user_token.token,
                        username: user_token.username,
                    };

                    // [FIX] 将身份信息注入到请求 extensions 中，而不是响应
                    // 这样 monitor_middleware 在处理请求时就能获取到 identity
                    // 因为中间件执行顺序：auth (外层) -> monitor (内层) -> handler
                    // 响应返回时：handler -> monitor -> auth
                    // 如果注入到 response，monitor 执行时 identity 还不存在
                    let (mut parts, body) = request.into_parts();
                    parts.extensions.insert(identity);
                    let request = Request::from_parts(parts, body);

                    // 执行请求
                    let response = next.run(request).await;

                    Ok(response)
                } else {
                    Err(StatusCode::UNAUTHORIZED)
                }
            }
            Ok((false, reason)) => {
                let reason_str = reason.unwrap_or_else(|| "Access denied".to_string());
                tracing::warn!("UserToken rejected: {}", reason_str);
                let body = serde_json::json!({
                    "error": {
                        "message": reason_str,
                        "type": "token_rejected",
                        "code": "token_rejected"
                    }
                });
                let response = axum::response::Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .header("Content-Type", "application/json")
                    .body(axum::body::Body::from(
                        serde_json::to_string(&body).unwrap(),
                    ))
                    .unwrap();
                Ok(response)
            }
            Err(e) => {
                tracing::error!("UserToken validation error: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// 用户令牌身份信息 (传递给 Monitor 使用)
#[derive(Clone, Debug)]
pub struct UserTokenIdentity {
    pub token_id: String,
    #[allow(dead_code)] // 保留原始 token 便于审计/调试
    pub token: String,
    pub username: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request as HttpRequest;
    use axum::routing::get;
    use tower::ServiceExt;

    fn req(uri: &str, headers: &[(&str, &str)]) -> HttpRequest<()> {
        let mut builder = HttpRequest::builder().uri(uri);
        for (k, v) in headers {
            builder = builder.header(*k, *v);
        }
        builder.body(()).unwrap()
    }

    #[test]
    fn task_extract_api_key_header_sources() {
        let r = req("/v1/models", &[("Authorization", "Bearer sk-bearer")]);
        assert_eq!(extract_api_key(&r, true).as_deref(), Some("sk-bearer"));

        let r = req("/v1/models", &[("Authorization", "bearer sk-lower")]);
        assert_eq!(extract_api_key(&r, true).as_deref(), Some("sk-lower"));

        let r = req("/v1/models", &[("Authorization", "sk-raw")]);
        assert_eq!(extract_api_key(&r, true).as_deref(), Some("sk-raw"));

        let r = req("/v1/models", &[("x-api-key", "sk-x-api-key")]);
        assert_eq!(extract_api_key(&r, true).as_deref(), Some("sk-x-api-key"));

        let r = req("/v1/models", &[("x-goog-api-key", "AIzaHeader")]);
        assert_eq!(extract_api_key(&r, true).as_deref(), Some("AIzaHeader"));

        // allow_query must not change header handling.
        let r = req("/v1/models", &[("x-goog-api-key", "AIzaHeader")]);
        assert_eq!(extract_api_key(&r, false).as_deref(), Some("AIzaHeader"));
    }

    #[test]
    fn task_extract_api_key_empty_values_fall_through() {
        let r = req("/v1/models", &[("Authorization", "Bearer   ")]);
        assert_eq!(extract_api_key(&r, true), None);

        // An empty Authorization must not mask a usable x-goog-api-key.
        let r = req(
            "/v1/models",
            &[("Authorization", "Bearer "), ("x-goog-api-key", "AIzaNext")],
        );
        assert_eq!(extract_api_key(&r, true).as_deref(), Some("AIzaNext"));

        assert_eq!(
            extract_api_key(&req("/v1beta/models?key=", &[]), true),
            None
        );
        assert_eq!(
            extract_api_key(&req("/v1beta/models?key=%20%20", &[]), true),
            None
        );
    }

    #[test]
    fn task_extract_api_key_query_only_when_allowed() {
        let r = req("/v1beta/models?key=AIzaQuery", &[]);
        assert_eq!(extract_api_key(&r, true).as_deref(), Some("AIzaQuery"));
        assert_eq!(extract_api_key(&r, false), None);

        let r = req(
            "/v1beta/models/gemini-2.5-flash:streamGenerateContent?alt=sse&key=AIzaMixed&x=1",
            &[],
        );
        assert_eq!(extract_api_key(&r, true).as_deref(), Some("AIzaMixed"));

        // Percent-encoded value is decoded.
        let r = req("/v1beta/models?key=sk%2Dabc", &[]);
        assert_eq!(extract_api_key(&r, true).as_deref(), Some("sk-abc"));

        // Only the exact `key` parameter counts.
        let r = req("/v1beta/models?api_key=nope&token=nope", &[]);
        assert_eq!(extract_api_key(&r, true), None);
    }

    #[test]
    fn task_extract_api_key_header_wins_over_query() {
        let r = req(
            "/v1beta/models?key=query-key",
            &[("x-goog-api-key", "header-key")],
        );
        assert_eq!(extract_api_key(&r, true).as_deref(), Some("header-key"));
    }

    fn security(admin_password: Option<&str>) -> Arc<RwLock<ProxySecurityConfig>> {
        Arc::new(RwLock::new(ProxySecurityConfig {
            auth_mode: ProxyAuthMode::Strict,
            api_key: "valid-proxy-key".to_string(),
            admin_password: admin_password.map(|s| s.to_string()),
            allow_lan_access: true,
            port: 8045,
            security_monitor: crate::proxy::config::SecurityMonitorConfig::default(),
        }))
    }

    async fn status_of(app: &axum::Router, uri: &str, headers: &[(&str, &str)]) -> StatusCode {
        let mut builder = HttpRequest::builder().uri(uri);
        for (k, v) in headers {
            builder = builder.header(*k, *v);
        }
        let request = builder.body(axum::body::Body::empty()).unwrap();
        app.clone().oneshot(request).await.unwrap().status()
    }

    #[tokio::test]
    async fn task_proxy_auth_accepts_gemini_query_key() {
        let app = axum::Router::new()
            .route("/v1beta/models", get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn_with_state(
                security(None),
                auth_middleware,
            ));

        assert_eq!(
            status_of(&app, "/v1beta/models?key=valid-proxy-key", &[]).await,
            StatusCode::OK
        );
        assert_eq!(
            status_of(&app, "/v1beta/models?alt=sse&key=valid-proxy-key&x=1", &[]).await,
            StatusCode::OK
        );
        assert_eq!(
            status_of(
                &app,
                "/v1beta/models",
                &[("x-goog-api-key", "valid-proxy-key")]
            )
            .await,
            StatusCode::OK
        );
        assert_eq!(
            status_of(&app, "/v1beta/models", &[]).await,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            status_of(&app, "/v1beta/models?key=", &[]).await,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn task_admin_auth_rejects_query_key() {
        // Dedicated admin password.
        let app = axum::Router::new()
            .route("/accounts", get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn_with_state(
                security(Some("admin-secret")),
                admin_auth_middleware,
            ));
        assert_eq!(
            status_of(&app, "/accounts?key=admin-secret", &[]).await,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            status_of(&app, "/accounts?key=valid-proxy-key", &[]).await,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            status_of(&app, "/accounts", &[("x-api-key", "admin-secret")]).await,
            StatusCode::OK
        );
        assert_eq!(
            status_of(
                &app,
                "/accounts",
                &[("Authorization", "Bearer admin-secret")]
            )
            .await,
            StatusCode::OK
        );

        // No admin password: falls back to api_key, still header-only.
        let app = axum::Router::new()
            .route("/accounts", get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn_with_state(
                security(None),
                admin_auth_middleware,
            ));
        assert_eq!(
            status_of(&app, "/accounts?key=valid-proxy-key", &[]).await,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            status_of(&app, "/accounts", &[("x-api-key", "valid-proxy-key")]).await,
            StatusCode::OK
        );

        // A header credential still wins even when a bogus query key is present.
        assert_eq!(
            status_of(
                &app,
                "/accounts?key=garbage",
                &[("x-api-key", "valid-proxy-key")]
            )
            .await,
            StatusCode::OK
        );
    }
}
