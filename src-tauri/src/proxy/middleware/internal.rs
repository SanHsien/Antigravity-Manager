use axum::{
    extract::{ConnectInfo, Request},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::net::{IpAddr, SocketAddr};

/// Returns true only for the `/internal` namespace boundary.
pub fn is_internal_path(path: &str) -> bool {
    path == "/internal" || path.starts_with("/internal/")
}

/// Requires internal requests to originate from the TCP loopback peer.
///
/// The proxy server injects `ConnectInfo<SocketAddr>` from the accepted TCP
/// connection. Forwarded headers are deliberately not consulted here.
pub fn require_internal_loopback(request: &Request) -> Result<(), StatusCode> {
    if !is_internal_path(request.uri().path()) {
        return Ok(());
    }

    let is_loopback = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .is_some_and(|ConnectInfo(peer)| is_loopback_peer(*peer));

    if is_loopback {
        Ok(())
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

/// Enforces the TCP loopback boundary before outer layers can short-circuit
/// internal requests, including CORS preflight.
pub async fn internal_loopback_middleware(request: Request, next: Next) -> Response {
    if let Err(status) = require_internal_loopback(&request) {
        return status.into_response();
    }

    next.run(request).await
}

fn is_loopback_peer(peer: SocketAddr) -> bool {
    match peer.ip() {
        IpAddr::V4(ip) => ip.is_loopback(),
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip
                    .to_ipv4_mapped()
                    .is_some_and(|mapped| mapped.is_loopback())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::{
        config::SecurityMonitorConfig,
        middleware::{
            auth::auth_middleware, cors::cors_layer,
            service_status::service_status_middleware_with_running,
        },
        ProxyAuthMode, ProxySecurityConfig,
    };
    use axum::{
        body::Body,
        extract::State,
        http::{header, Method},
        middleware,
        routing::any,
        Router,
    };
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use tokio::sync::RwLock;
    use tower::ServiceExt;

    async fn counted_handler(State(counter): State<Arc<AtomicUsize>>) -> StatusCode {
        counter.fetch_add(1, Ordering::SeqCst);
        StatusCode::NO_CONTENT
    }

    fn security(mode: ProxyAuthMode) -> Arc<RwLock<ProxySecurityConfig>> {
        Arc::new(RwLock::new(ProxySecurityConfig {
            auth_mode: mode,
            api_key: "test-key".to_string(),
            admin_password: None,
            allow_lan_access: false,
            port: 8045,
            security_monitor: SecurityMonitorConfig::default(),
        }))
    }

    fn request(path: &str, method: Method, peer: Option<SocketAddr>) -> Request {
        let mut request = Request::builder()
            .method(method)
            .uri(path)
            .header("x-forwarded-for", "127.0.0.1")
            .header("x-real-ip", "::1")
            .body(Body::empty())
            .unwrap();

        if let Some(peer) = peer {
            request.extensions_mut().insert(ConnectInfo(peer));
        }

        request
    }

    fn auth_router(counter: Arc<AtomicUsize>, mode: ProxyAuthMode) -> Router {
        Router::new()
            .route("/internal/warmup", any(counted_handler))
            .route("/internal-warmup", any(counted_handler))
            .route("/health", any(counted_handler))
            .route("/v1/normal", any(counted_handler))
            .with_state(counter)
            .layer(middleware::from_fn_with_state(
                security(mode),
                auth_middleware,
            ))
    }

    fn stopped_service_router(counter: Arc<AtomicUsize>) -> Router {
        Router::new()
            .route("/internal/warmup", any(counted_handler))
            .route("/health", any(counted_handler))
            .with_state(counter)
            .layer(middleware::from_fn(|request, next| {
                service_status_middleware_with_running(false, request, next)
            }))
    }

    /// Mirrors the server's effective relevant order:
    /// BodyLimit -> internal guard -> CORS -> service status -> routes.
    fn cors_guard_router(counter: Arc<AtomicUsize>, running: bool) -> Router {
        Router::new()
            .route("/internal/warmup", any(counted_handler))
            .route("/v1/normal", any(counted_handler))
            .with_state(counter)
            .layer(middleware::from_fn(|request, next| {
                service_status_middleware_with_running(running, request, next)
            }))
            .layer(cors_layer())
            .layer(middleware::from_fn(internal_loopback_middleware))
    }

    fn preflight_request(path: &str, peer: Option<SocketAddr>) -> Request {
        let mut request = request(path, Method::OPTIONS, peer);
        request
            .headers_mut()
            .insert(header::ORIGIN, "https://example.test".parse().unwrap());
        request.headers_mut().insert(
            header::ACCESS_CONTROL_REQUEST_METHOD,
            Method::POST.as_str().parse().unwrap(),
        );
        request
    }

    #[tokio::test]
    async fn auth_guard_rejects_external_peer_when_auth_is_off_and_headers_are_forged() {
        let counter = Arc::new(AtomicUsize::new(0));
        let mut external = request(
            "/internal/warmup",
            Method::POST,
            Some("198.51.100.8:8045".parse().unwrap()),
        );
        external
            .headers_mut()
            .insert("authorization", "Bearer test-key".parse().unwrap());

        let response = auth_router(counter.clone(), ProxyAuthMode::Off)
            .oneshot(external)
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn auth_guard_rejects_internal_options_before_cors_bypass() {
        let counter = Arc::new(AtomicUsize::new(0));
        let response = auth_router(counter.clone(), ProxyAuthMode::Off)
            .oneshot(request(
                "/internal/warmup",
                Method::OPTIONS,
                Some("198.51.100.8:8045".parse().unwrap()),
            ))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn auth_guard_rejects_missing_and_mapped_external_peers() {
        let counter = Arc::new(AtomicUsize::new(0));
        let app = auth_router(counter.clone(), ProxyAuthMode::Off);

        let missing = app
            .clone()
            .oneshot(request("/internal/warmup", Method::POST, None))
            .await
            .unwrap();
        let mapped_external = app
            .oneshot(request(
                "/internal/warmup",
                Method::POST,
                Some("[::ffff:198.51.100.8]:8045".parse().unwrap()),
            ))
            .await
            .unwrap();

        assert_eq!(missing.status(), StatusCode::FORBIDDEN);
        assert_eq!(mapped_external.status(), StatusCode::FORBIDDEN);
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn auth_guard_allows_ipv4_ipv6_and_mapped_loopback_peers() {
        let counter = Arc::new(AtomicUsize::new(0));
        let app = auth_router(counter.clone(), ProxyAuthMode::Off);

        for peer in ["127.0.0.1:8045", "[::1]:8045", "[::ffff:127.0.0.1]:8045"] {
            let response = app
                .clone()
                .oneshot(request(
                    "/internal/warmup",
                    Method::POST,
                    Some(peer.parse().unwrap()),
                ))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NO_CONTENT, "peer={peer}");
        }

        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn service_status_guard_denies_external_before_stopped_bypass() {
        let counter = Arc::new(AtomicUsize::new(0));
        let app = stopped_service_router(counter.clone());

        let external = app
            .clone()
            .oneshot(request(
                "/internal/warmup",
                Method::POST,
                Some("198.51.100.8:8045".parse().unwrap()),
            ))
            .await
            .unwrap();
        let loopback = app
            .oneshot(request(
                "/internal/warmup",
                Method::POST,
                Some("127.0.0.1:8045".parse().unwrap()),
            ))
            .await
            .unwrap();

        assert_eq!(external.status(), StatusCode::FORBIDDEN);
        assert_eq!(loopback.status(), StatusCode::NO_CONTENT);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn cors_preflight_rejects_internal_external_missing_and_mapped_external_peers() {
        let counter = Arc::new(AtomicUsize::new(0));
        let app = cors_guard_router(counter.clone(), false);

        for peer in [
            Some("198.51.100.8:8045".parse().unwrap()),
            None,
            Some("[::ffff:198.51.100.8]:8045".parse().unwrap()),
        ] {
            let response = app
                .clone()
                .oneshot(preflight_request("/internal/warmup", peer))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN);
            assert!(response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .is_none());
        }

        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn cors_preflight_allows_loopback_internal_and_preserves_cors_headers() {
        let counter = Arc::new(AtomicUsize::new(0));
        let app = cors_guard_router(counter.clone(), false);

        for peer in ["127.0.0.1:8045", "[::1]:8045", "[::ffff:127.0.0.1]:8045"] {
            let response = app
                .clone()
                .oneshot(preflight_request(
                    "/internal/warmup",
                    Some(peer.parse().unwrap()),
                ))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "peer={peer}");
            assert_eq!(
                response
                    .headers()
                    .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                    .unwrap(),
                "*",
                "peer={peer}"
            );
        }

        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn noninternal_preflight_remains_cors_enabled_when_service_is_stopped() {
        let counter = Arc::new(AtomicUsize::new(0));
        let response = cors_guard_router(counter.clone(), false)
            .oneshot(preflight_request("/v1/normal", None))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
                .unwrap(),
            "*"
        );
        assert_eq!(counter.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn internal_namespace_boundary_and_noninternal_health_and_options_are_unchanged() {
        let counter = Arc::new(AtomicUsize::new(0));
        let app = auth_router(counter.clone(), ProxyAuthMode::Strict);

        let boundary = app
            .clone()
            .oneshot(request(
                "/internal-warmup",
                Method::POST,
                Some("198.51.100.8:8045".parse().unwrap()),
            ))
            .await
            .unwrap();
        let health = app
            .clone()
            .oneshot(request("/health", Method::GET, None))
            .await
            .unwrap();
        let options = app
            .oneshot(request("/v1/normal", Method::OPTIONS, None))
            .await
            .unwrap();

        assert_eq!(boundary.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(health.status(), StatusCode::NO_CONTENT);
        assert_eq!(options.status(), StatusCode::NO_CONTENT);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }
}
