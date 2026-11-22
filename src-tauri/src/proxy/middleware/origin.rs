// Browser `Origin` guard for the proxy routes when authentication is effectively off.
//
// With `auth_mode = off` (the default `auto` resolves to `off` while LAN access is
// disabled) the proxy accepts every request that reaches the loopback port. A web page the
// user visits can reach that port too: the browser sends the request first and applies CORS
// afterwards, and a WebSocket upgrade is not subject to CORS at all. Such a page could spend
// the user's account quota and, with a permissive CORS policy, read the answer.
//
// A browser always attaches `Origin` to cross-origin `fetch`/XHR, to every non-GET form
// submission and to every WebSocket handshake, and page script cannot forge or remove it.
// Command-line clients (Claude Code, Codex, curl, SDKs) send no `Origin`. So when no
// credential is required:
//
//   no `Origin` header               -> allowed (non-browser client)
//   loopback or the app's own origin -> allowed
//   anything else, including `null`  -> 403
//
// When authentication is on, the credential is the gate and this guard does nothing.
use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use std::net::IpAddr;
use std::sync::{Arc, OnceLock};
use tokio::sync::RwLock;

use crate::proxy::{ProxyAuthMode, ProxySecurityConfig};

/// Comma-separated extra origins (exact `scheme://host[:port]`) to trust while auth is off,
/// for a headless deployment or a local tool served from a non-loopback origin.
pub const EXTRA_ORIGINS_ENV: &str = "ABV_ALLOWED_ORIGINS";

/// Canonical form used to compare origins: trimmed, lowercased, without a trailing slash.
fn normalize_origin(origin: &str) -> String {
    origin.trim().trim_end_matches('/').to_ascii_lowercase()
}

fn is_loopback_host(host: &url::Host<&str>) -> bool {
    match host {
        url::Host::Domain(name) => {
            let name = name.trim_end_matches('.').to_ascii_lowercase();
            // RFC 6761: `localhost` and every name under it are loopback. This also covers
            // the Tauri webview origins `http(s)://tauri.localhost` used on Windows/Android.
            if name == "localhost" || name.ends_with(".localhost") {
                return true;
            }
            // A non-special scheme keeps an IP literal as an opaque domain string.
            name.parse::<IpAddr>()
                .map(|ip| ip.is_loopback())
                .unwrap_or(false)
        }
        url::Host::Ipv4(ip) => ip.is_loopback(),
        url::Host::Ipv6(ip) => {
            ip.is_loopback()
                || ip
                    .to_ipv4_mapped()
                    .map(|v4| v4.is_loopback())
                    .unwrap_or(false)
        }
    }
}

/// True for an origin that only software on this machine can have: a loopback web origin
/// (any port) or the desktop app's own webview origin.
///
/// The app's frontend is served from `tauri://localhost` (macOS/Linux),
/// `http://tauri.localhost` (Windows/Android, `https://` with `useHttpsScheme`) and
/// `http://localhost:1420` in `tauri dev`.
pub fn is_local_app_origin(origin: &str) -> bool {
    let origin = origin.trim();
    // `null` is what sandboxed iframes, `data:` documents and local files send; any web page
    // can produce it, so it is never trusted.
    if origin.is_empty() || origin.eq_ignore_ascii_case("null") {
        return false;
    }
    let Ok(parsed) = url::Url::parse(origin) else {
        return false;
    };
    // An origin is `scheme://host[:port]` only.
    if !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
        || !matches!(parsed.path(), "" | "/")
    {
        return false;
    }
    let Some(host) = parsed.host() else {
        return false;
    };
    match parsed.scheme() {
        "http" | "https" => is_loopback_host(&host),
        "tauri" => {
            matches!(host, url::Host::Domain(name) if name.eq_ignore_ascii_case("localhost"))
        }
        _ => false,
    }
}

fn parse_extra_origins(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(normalize_origin)
        // `*` and `null` would switch the guard off for every web page.
        .filter(|o| !o.is_empty() && o != "*" && o != "null")
        .collect()
}

fn extra_allowed_origins() -> &'static [String] {
    static EXTRA: OnceLock<Vec<String>> = OnceLock::new();
    EXTRA.get_or_init(|| {
        let extra = std::env::var(EXTRA_ORIGINS_ENV)
            .map(|raw| parse_extra_origins(&raw))
            .unwrap_or_default();
        if !extra.is_empty() {
            tracing::info!(
                "{}: trusting extra browser origins while auth is off: {:?}",
                EXTRA_ORIGINS_ENV,
                extra
            );
        }
        extra
    })
}

fn is_trusted_origin_with(origin: &str, extra: &[String]) -> bool {
    is_local_app_origin(origin) || extra.contains(&normalize_origin(origin))
}

/// Whether a browser origin may use the proxy without a credential.
pub fn is_trusted_origin(origin: &str) -> bool {
    is_trusted_origin_with(origin, extra_allowed_origins())
}

/// Whether the request may use the proxy without a credential, judged by `Origin` alone.
/// A request without `Origin` passes; every `Origin` value present must be trusted.
pub fn request_origin_is_trusted(headers: &HeaderMap) -> bool {
    headers
        .get_all(header::ORIGIN)
        .iter()
        .all(|value| value.to_str().map(is_trusted_origin).unwrap_or(false))
}

/// True when the proxy routes accept requests without a credential.
pub fn proxy_auth_is_off(security: &ProxySecurityConfig) -> bool {
    matches!(security.effective_auth_mode(), ProxyAuthMode::Off)
}

fn forbidden_origin_response() -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({
            "error": {
                "message": "Cross-origin browser request rejected: API key authentication is off, so only loopback origins and the desktop app may call this proxy from a browser. Turn on API key authentication (auth_mode) to call it from another web origin, or list the origin in ABV_ALLOWED_ORIGINS.",
                "type": "origin_not_allowed",
                "code": "origin_not_allowed"
            }
        })),
    )
        .into_response()
}

/// Reject browser requests from untrusted origins on the proxy routes while auth is off.
///
/// Covers plain HTTP and the WebSocket upgrade (`GET /v1/responses`): the upgrade is an
/// ordinary request at this point and browsers always send `Origin` on it.
pub async fn origin_guard_middleware(
    State(security): State<Arc<RwLock<ProxySecurityConfig>>>,
    request: Request,
    next: Next,
) -> Response {
    if request.headers().contains_key(header::ORIGIN) {
        let auth_off = proxy_auth_is_off(&*security.read().await);
        if auth_off && !request_origin_is_trusted(request.headers()) {
            tracing::warn!(
                "Rejected cross-origin browser request while auth is off: {} {} (Origin: {:?})",
                request.method(),
                request.uri().path(),
                request.headers().get(header::ORIGIN)
            );
            return forbidden_origin_response();
        }
    }
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::extract::ws::WebSocketUpgrade;
    use axum::http::Request as HttpRequest;
    use axum::routing::{get, post};
    use axum::Router;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tower::ServiceExt;

    fn security(
        auth_mode: ProxyAuthMode,
        allow_lan_access: bool,
    ) -> Arc<RwLock<ProxySecurityConfig>> {
        Arc::new(RwLock::new(ProxySecurityConfig {
            auth_mode,
            api_key: "sk-test".to_string(),
            admin_password: None,
            allow_lan_access,
            port: 8045,
            security_monitor: crate::proxy::config::SecurityMonitorConfig::default(),
        }))
    }

    /// The proxy routes that matter here, behind the guard and the real CORS layer, in the
    /// same order as `server.rs` (CORS outside, guard inside).
    fn app(security: Arc<RwLock<ProxySecurityConfig>>) -> Router {
        Router::new()
            .route("/v1/messages", post(|| async { "spent quota" }))
            .route(
                "/v1/responses",
                get(|ws: WebSocketUpgrade| async move { ws.on_upgrade(|_socket| async {}) }),
            )
            .layer(axum::middleware::from_fn_with_state(
                security.clone(),
                origin_guard_middleware,
            ))
            .layer(crate::proxy::middleware::cors::cors_layer(security))
    }

    async fn post_status(app: &Router, origin: Option<&str>) -> (StatusCode, Option<String>) {
        let mut builder = HttpRequest::builder()
            .method("POST")
            .uri("/v1/messages")
            // A "simple" request: no preflight, so CORS alone never stops it.
            .header("content-type", "text/plain");
        if let Some(origin) = origin {
            builder = builder.header("origin", origin);
        }
        let response = app
            .clone()
            .oneshot(builder.body(Body::from("{}")).unwrap())
            .await
            .unwrap();
        let allow_origin = response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        (response.status(), allow_origin)
    }

    async fn preflight_allow_origin(app: &Router, origin: &str) -> Option<String> {
        let request = HttpRequest::builder()
            .method("OPTIONS")
            .uri("/v1/messages")
            .header("origin", origin)
            .header("access-control-request-method", "POST")
            .header("access-control-request-headers", "content-type")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        response
            .headers()
            .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string)
    }

    /// Run a real WebSocket handshake against the app and return the HTTP status line.
    async fn ws_handshake_status(app: Router, origin: Option<&str>) -> String {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let mut handshake = format!(
            "GET /v1/responses HTTP/1.1\r\nHost: {addr}\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n"
        );
        if let Some(origin) = origin {
            handshake.push_str(&format!("Origin: {origin}\r\n"));
        }
        handshake.push_str("\r\n");
        stream.write_all(handshake.as_bytes()).await.unwrap();

        let mut received = Vec::new();
        let mut buf = [0u8; 512];
        while !received.windows(2).any(|w| w == b"\r\n") {
            let n = tokio::time::timeout(std::time::Duration::from_secs(10), stream.read(&mut buf))
                .await
                .expect("handshake response timed out")
                .unwrap();
            if n == 0 {
                break;
            }
            received.extend_from_slice(&buf[..n]);
        }
        server.abort();
        String::from_utf8_lossy(&received)
            .lines()
            .next()
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn task_only_loopback_and_the_desktop_app_are_local_origins() {
        for origin in [
            "tauri://localhost",
            "http://tauri.localhost",
            "https://tauri.localhost",
            "http://localhost:1420",
            "http://localhost",
            "HTTP://LOCALHOST:3000",
            "http://127.0.0.1:8045",
            "http://127.8.9.10",
            "http://[::1]:8045",
            "https://app.localhost:8443",
        ] {
            assert!(is_local_app_origin(origin), "{origin} must be trusted");
        }
        for origin in [
            "null",
            "",
            "https://evil.example",
            "http://evil.example:8045",
            // Look-alikes of loopback names and addresses.
            "http://localhost.evil.example",
            "http://127.0.0.1.evil.example",
            "http://tauri.localhost.evil.example",
            "http://notlocalhost",
            "http://evil.example#http://localhost",
            "http://localhost@evil.example",
            "http://192.168.1.10:8045",
            "http://0.0.0.0:8045",
            "tauri://evil.example",
            "file://",
            "chrome-extension://abcdefghijklmnop",
            "not a url",
        ] {
            assert!(!is_local_app_origin(origin), "{origin} must not be trusted");
        }
    }

    #[test]
    fn task_extra_origins_match_exactly_and_never_wildcard() {
        let extra =
            parse_extra_origins(" https://chat.example.com/ , *, null, ,HTTP://NAS.lan:3000");
        assert_eq!(
            extra,
            vec!["https://chat.example.com", "http://nas.lan:3000"]
        );
        assert!(is_trusted_origin_with("https://chat.example.com", &extra));
        assert!(is_trusted_origin_with("http://nas.lan:3000", &extra));
        assert!(!is_trusted_origin_with(
            "https://chat.example.com.evil.example",
            &extra
        ));
        assert!(!is_trusted_origin_with("https://evil.example", &extra));
        assert!(!is_trusted_origin_with("null", &extra));
    }

    #[test]
    fn task_request_without_origin_passes_and_any_untrusted_origin_fails() {
        let mut headers = HeaderMap::new();
        assert!(request_origin_is_trusted(&headers));
        headers.append(header::ORIGIN, "http://localhost:1420".parse().unwrap());
        assert!(request_origin_is_trusted(&headers));
        // A second, untrusted value must not hide behind a trusted one.
        headers.append(header::ORIGIN, "https://evil.example".parse().unwrap());
        assert!(!request_origin_is_trusted(&headers));
    }

    #[tokio::test]
    async fn task_auth_off_rejects_evil_origin_on_http_and_keeps_cli_and_app_working() {
        // The shipped default: `auto` with LAN access off resolves to `off`.
        let app = app(security(ProxyAuthMode::Auto, false));

        // A web page: rejected before the handler runs, and not readable either.
        let (status, allow_origin) = post_status(&app, Some("https://evil.example")).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(allow_origin, None);
        let (status, allow_origin) = post_status(&app, Some("null")).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(allow_origin, None);
        assert_eq!(
            preflight_allow_origin(&app, "https://evil.example").await,
            None
        );

        // CLI clients send no Origin.
        assert_eq!(post_status(&app, None).await.0, StatusCode::OK);

        // The desktop app's own webview and a loopback page.
        for origin in [
            "tauri://localhost",
            "http://tauri.localhost",
            "http://localhost:1420",
        ] {
            let (status, allow_origin) = post_status(&app, Some(origin)).await;
            assert_eq!(status, StatusCode::OK, "{origin}");
            assert_eq!(allow_origin.as_deref(), Some(origin));
            assert_eq!(
                preflight_allow_origin(&app, origin).await.as_deref(),
                Some(origin)
            );
        }
    }

    #[tokio::test]
    async fn task_auth_on_keeps_cross_origin_behaviour() {
        // With a credential required, the origin is not the gate: any origin may call, as
        // before (the real auth layer, not part of this router, checks the key).
        for security in [
            security(ProxyAuthMode::Strict, false),
            security(ProxyAuthMode::AllExceptHealth, false),
            security(ProxyAuthMode::Auto, true),
        ] {
            let app = app(security);
            let (status, allow_origin) = post_status(&app, Some("https://chat.example.com")).await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(allow_origin.as_deref(), Some("https://chat.example.com"));
            assert_eq!(
                preflight_allow_origin(&app, "https://chat.example.com")
                    .await
                    .as_deref(),
                Some("https://chat.example.com")
            );
        }
    }

    #[tokio::test]
    async fn task_auth_off_rejects_evil_origin_on_websocket_upgrade() {
        let status = ws_handshake_status(
            app(security(ProxyAuthMode::Auto, false)),
            Some("https://evil.example"),
        )
        .await;
        assert!(status.starts_with("HTTP/1.1 403"), "got {status:?}");
    }

    #[tokio::test]
    async fn task_websocket_upgrade_works_without_origin_and_from_the_app() {
        // Codex CLI: no Origin.
        let status = ws_handshake_status(app(security(ProxyAuthMode::Auto, false)), None).await;
        assert!(status.starts_with("HTTP/1.1 101"), "got {status:?}");

        let status = ws_handshake_status(
            app(security(ProxyAuthMode::Auto, false)),
            Some("tauri://localhost"),
        )
        .await;
        assert!(status.starts_with("HTTP/1.1 101"), "got {status:?}");

        // Auth on: the origin is not the gate.
        let status = ws_handshake_status(
            app(security(ProxyAuthMode::Strict, false)),
            Some("https://chat.example.com"),
        )
        .await;
        assert!(status.starts_with("HTTP/1.1 101"), "got {status:?}");
    }
}
