// CORS 中间件
use axum::http::Method;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

use crate::proxy::middleware::origin::{is_trusted_origin, proxy_auth_is_off};
use crate::proxy::ProxySecurityConfig;

/// Management API paths (`/api/...`). They always require a credential, whatever `auth_mode`
/// says, so the origin is never what protects them.
fn is_admin_path(path: &str) -> bool {
    path == "/api" || path.starts_with("/api/")
}

/// 创建 CORS layer
///
/// While a credential is required, any origin may call (as before): a page that does not
/// know the key gets 401. While the proxy routes are open (`auth_mode` effectively `off`),
/// only loopback origins and the desktop app's own origin are allowed, so a web page the
/// user visits can neither pass a preflight nor read a response. The matching server-side
/// rejection is `origin::origin_guard_middleware`; CORS alone does not stop a "simple"
/// request from being processed.
pub fn cors_layer(security: Arc<RwLock<ProxySecurityConfig>>) -> CorsLayer {
    let allow_origin = AllowOrigin::predicate(move |origin, parts| {
        if is_admin_path(parts.uri.path()) {
            return true;
        }
        // The predicate is synchronous. The lock is only write-held for the instant a config
        // save swaps the value; if that collides, answer as if auth were off (the strict side).
        let auth_off = security
            .try_read()
            .map(|s| proxy_auth_is_off(&s))
            .unwrap_or(true);
        if !auth_off {
            return true;
        }
        origin.to_str().map(is_trusted_origin).unwrap_or(false)
    });

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::HEAD,
            Method::OPTIONS,
            Method::PATCH,
        ])
        .allow_headers(Any)
        .allow_credentials(false)
        .max_age(std::time::Duration::from_secs(3600))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::ProxyAuthMode;
    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    fn open_proxy() -> Arc<RwLock<ProxySecurityConfig>> {
        Arc::new(RwLock::new(ProxySecurityConfig {
            auth_mode: ProxyAuthMode::Off,
            api_key: "sk-test".to_string(),
            admin_password: None,
            allow_lan_access: false,
            port: 8045,
            security_monitor: crate::proxy::config::SecurityMonitorConfig::default(),
        }))
    }

    async fn allow_origin_for(app: &Router, uri: &str, origin: &str) -> Option<String> {
        let request = Request::builder()
            .uri(uri)
            .header("origin", origin)
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    }

    #[tokio::test]
    async fn task_open_proxy_does_not_expose_responses_to_other_origins() {
        let app = Router::new()
            .route("/v1/models", get(|| async { "models" }))
            .route("/api/accounts", get(|| async { "accounts" }))
            .route("/apiary", get(|| async { "not admin" }))
            .layer(cors_layer(open_proxy()));

        assert_eq!(
            allow_origin_for(&app, "/v1/models", "https://evil.example").await,
            None
        );
        assert_eq!(
            allow_origin_for(&app, "/v1/models", "http://localhost:1420")
                .await
                .as_deref(),
            Some("http://localhost:1420")
        );
        // The management API keeps its own credential check, so its CORS is unchanged.
        assert_eq!(
            allow_origin_for(&app, "/api/accounts", "https://panel.example")
                .await
                .as_deref(),
            Some("https://panel.example")
        );
        // Only the `/api` path segment counts as the management API.
        assert_eq!(
            allow_origin_for(&app, "/apiary", "https://evil.example").await,
            None
        );
    }
}
