//! HTTP API Module
//! Provides local HTTP interfaces for external programs (e.g., VS Code extension) to call.
//!
//! The server is OFF by default and only starts when `local_account_api.json` says
//! `"enabled": true` (see [`spawn_server_if_enabled`]). It binds `127.0.0.1` only and has no
//! credentials, so once it is on, any local process can list accounts and switch the active
//! one. Browser-originated requests are refused (see [`is_local_caller`]).

use axum::{
    extract::{Path, Query, Request, State},
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;
use std::path::Path as FsPath;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::modules::{account, logger, proxy_db};

/// Default port for HTTP API server
pub const DEFAULT_PORT: u16 = 19527;

// ============================================================================
// Settings
// ============================================================================

/// Settings file name inside the data directory.
const SETTINGS_FILE: &str = "local_account_api.json";

/// Lowest port the settings accept (same range the settings UI shows: 1024-65535).
pub const MIN_PORT: u16 = 1024;

/// HTTP API Settings
///
/// `enabled` defaults to `false` (also when the file or the key is missing): the maintainer
/// has to opt in explicitly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpApiSettings {
    /// Whether to enable HTTP API service
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Listening port
    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_enabled() -> bool {
    false
}

fn default_port() -> u16 {
    DEFAULT_PORT
}

impl Default for HttpApiSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            port: DEFAULT_PORT,
        }
    }
}

impl HttpApiSettings {
    /// Reject values the server could not (or should not) bind.
    pub fn validate(&self) -> Result<(), String> {
        if self.port < MIN_PORT {
            return Err(format!(
                "invalid_port: {} (range: {}-65535)",
                self.port, MIN_PORT
            ));
        }
        Ok(())
    }
}

/// Load HTTP API settings from `path`. A missing file means the defaults (disabled).
pub fn load_settings_from(path: &FsPath) -> Result<HttpApiSettings, String> {
    if !path.exists() {
        return Ok(HttpApiSettings::default());
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read settings file: {}", e))?;

    serde_json::from_str(&content).map_err(|e| format!("Failed to parse settings: {}", e))
}

/// Save HTTP API settings to `path` (validated first).
pub fn save_settings_to(path: &FsPath, settings: &HttpApiSettings) -> Result<(), String> {
    settings.validate()?;

    let content = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    std::fs::write(path, content).map_err(|e| format!("Failed to write settings file: {}", e))
}

fn settings_path() -> Result<std::path::PathBuf, String> {
    let data_dir = crate::modules::account::get_data_dir()
        .map_err(|e| format!("Failed to get data dir: {}", e))?;
    Ok(data_dir.join(SETTINGS_FILE))
}

/// Load HTTP API settings
pub fn load_settings() -> Result<HttpApiSettings, String> {
    load_settings_from(&settings_path()?)
}

/// Save HTTP API settings
pub fn save_settings(settings: &HttpApiSettings) -> Result<(), String> {
    save_settings_to(&settings_path()?, settings)
}

/// The port to listen on, or `None` when the server must not start (disabled or invalid port).
pub fn server_port_if_enabled(settings: &HttpApiSettings) -> Option<u16> {
    if settings.enabled && settings.validate().is_ok() {
        Some(settings.port)
    } else {
        None
    }
}

/// Server State
#[derive(Clone)]
pub struct ApiState {
    /// Whether there is a switch operation currently in progress
    pub switching: Arc<RwLock<bool>>,
    pub integration: crate::modules::integration::SystemManager,
}

impl ApiState {
    pub fn new(integration: crate::modules::integration::SystemManager) -> Self {
        Self {
            switching: Arc::new(RwLock::new(false)),
            integration,
        }
    }
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

#[derive(Serialize)]
struct AccountResponse {
    id: String,
    email: String,
    name: Option<String>,
    is_current: bool,
    disabled: bool,
    live_limited_models: HashMap<String, crate::models::account::LiveLimitStatus>,
    quota: Option<QuotaResponse>,
    device_bound: bool,
    last_used: i64,
}

#[derive(Serialize)]
struct QuotaResponse {
    models: Vec<ModelQuota>,
    updated_at: Option<i64>,
    subscription_tier: Option<String>,
}

#[derive(Serialize)]
struct ModelQuota {
    name: String,
    percentage: i32,
    reset_time: String,
}

#[derive(Serialize)]
struct AccountListResponse {
    accounts: Vec<AccountResponse>,
    current_account_id: Option<String>,
}

#[derive(Serialize)]
struct CurrentAccountResponse {
    account: Option<AccountResponse>,
}

#[derive(Serialize)]
struct SwitchResponse {
    success: bool,
    message: String,
}

#[derive(Serialize)]
struct RefreshResponse {
    success: bool,
    message: String,
    refreshed_count: usize,
}

#[derive(Serialize)]
struct BindDeviceResponse {
    success: bool,
    message: String,
    device_profile: Option<DeviceProfileResponse>,
}

#[derive(Serialize)]
struct DeviceProfileResponse {
    machine_id: String,
    mac_machine_id: String,
    dev_device_id: String,
    sqm_id: String,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Serialize)]
struct LogsResponse {
    total: u64,
    logs: Vec<crate::proxy::monitor::ProxyRequestLog>,
}

// ============================================================================
// Request Types
// ============================================================================

#[derive(Deserialize)]
struct SwitchRequest {
    account_id: String,
}

#[derive(Deserialize)]
struct BindDeviceRequest {
    #[serde(default = "default_bind_mode")]
    mode: String,
}

fn default_bind_mode() -> String {
    "generate".to_string()
}

#[derive(Deserialize)]
struct LogsRequest {
    #[serde(default)]
    limit: usize,
    #[serde(default)]
    offset: usize,
    #[serde(default)]
    filter: String,
    #[serde(default)]
    errors_only: bool,
}

// ============================================================================
// Handlers
// ============================================================================

/// GET /health - Health check
async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// GET /accounts - Get all accounts
async fn list_accounts() -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let accounts = account::list_accounts().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e }),
        )
    })?;

    let current_id = account::get_current_account_id().ok().flatten();

    let account_responses: Vec<AccountResponse> = accounts
        .into_iter()
        .map(|acc| {
            let is_current = current_id.as_ref().map(|id| id == &acc.id).unwrap_or(false);
            let quota = acc.quota.map(|q| QuotaResponse {
                models: q
                    .models
                    .into_iter()
                    .map(|m| ModelQuota {
                        name: m.name,
                        percentage: m.percentage,
                        reset_time: m.reset_time,
                    })
                    .collect(),
                updated_at: Some(q.last_updated),
                subscription_tier: q.subscription_tier,
            });

            AccountResponse {
                id: acc.id,
                email: acc.email,
                name: acc.name,
                is_current,
                disabled: acc.disabled,
                live_limited_models: acc.live_limited_models,
                quota,
                device_bound: acc.device_profile.is_some(),
                last_used: acc.last_used,
            }
        })
        .collect();

    Ok(Json(AccountListResponse {
        current_account_id: current_id,
        accounts: account_responses,
    }))
}

/// GET /accounts/current - Get current account
async fn get_current_account() -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let current = account::get_current_account().map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e }),
        )
    })?;

    let response = current.map(|acc| {
        let quota = acc.quota.map(|q| QuotaResponse {
            models: q
                .models
                .into_iter()
                .map(|m| ModelQuota {
                    name: m.name,
                    percentage: m.percentage,
                    reset_time: m.reset_time,
                })
                .collect(),
            updated_at: Some(q.last_updated),
            subscription_tier: q.subscription_tier,
        });

        AccountResponse {
            id: acc.id,
            email: acc.email,
            name: acc.name,
            is_current: true,
            disabled: acc.disabled,
            live_limited_models: acc.live_limited_models,
            quota,
            device_bound: acc.device_profile.is_some(),
            last_used: acc.last_used,
        }
    });

    Ok(Json(CurrentAccountResponse { account: response }))
}

/// Claim the single switch slot. `false` means another switch is already running.
async fn try_begin_switch(switching: &RwLock<bool>) -> bool {
    let mut guard = switching.write().await;
    if *guard {
        false
    } else {
        *guard = true;
        true
    }
}

/// POST /accounts/switch - Switch account
async fn switch_account(
    State(state): State<ApiState>,
    Json(payload): Json<SwitchRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    // Only one switch at a time: claim the slot atomically (check-then-set under two separate
    // locks let two simultaneous requests both pass and restart the desktop app twice).
    if !try_begin_switch(&state.switching).await {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrorResponse {
                error: "Another switch operation is already in progress".to_string(),
            }),
        ));
    }

    let account_id = payload.account_id.clone();
    let state_clone = state.clone();

    // Execute switch asynchronously (non-blocking response)
    tokio::spawn(async move {
        logger::log_info(&format!(
            "[HTTP API] Starting account switch: {}",
            account_id
        ));

        match account::switch_account(&account_id, None, &state_clone.integration).await {
            Ok(()) => {
                logger::log_info(&format!(
                    "[HTTP API] Account switch successful: {}",
                    account_id
                ));
            }
            Err(e) => {
                logger::log_error(&format!("[HTTP API] Account switch failed: {}", e));
            }
        }

        // Mark switch ended
        let mut switching = state_clone.switching.write().await;
        *switching = false;
    });

    // Immediately return 202 Accepted
    Ok((
        StatusCode::ACCEPTED,
        Json(SwitchResponse {
            success: true,
            message: format!("Account switch task started: {}", payload.account_id),
        }),
    ))
}

/// POST /accounts/refresh - Refresh all quotas
async fn refresh_all_quotas() -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    logger::log_info("[HTTP API] Starting refresh of all account quotas");

    // Execute refresh asynchronously
    tokio::spawn(async {
        match account::refresh_all_quotas_logic().await {
            Ok(stats) => {
                logger::log_info(&format!(
                    "[HTTP API] Quota refresh completed, successful {}/{} accounts",
                    stats.success, stats.total
                ));
            }
            Err(e) => {
                logger::log_error(&format!("[HTTP API] Quota refresh failed: {}", e));
            }
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(RefreshResponse {
            success: true,
            message: "Quota refresh task started".to_string(),
            refreshed_count: 0,
        }),
    ))
}

/// POST /accounts/:id/bind-device - Bind device fingerprint
async fn bind_device(
    Path(account_id): Path<String>,
    Json(payload): Json<BindDeviceRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    logger::log_info(&format!(
        "[HTTP API] Binding device fingerprint: account={}, mode={}",
        account_id, payload.mode
    ));

    let result = account::bind_device_profile(&account_id, &payload.mode).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e }),
        )
    })?;

    Ok(Json(BindDeviceResponse {
        success: true,
        message: "Device fingerprint bound successfully".to_string(),
        device_profile: Some(DeviceProfileResponse {
            machine_id: result.machine_id,
            mac_machine_id: result.mac_machine_id,
            dev_device_id: result.dev_device_id,
            sqm_id: result.sqm_id,
        }),
    }))
}

/// GET /logs - Get proxy logs
async fn get_logs(
    Query(params): Query<LogsRequest>,
) -> Result<impl IntoResponse, (StatusCode, Json<ErrorResponse>)> {
    let limit = if params.limit == 0 { 50 } else { params.limit };

    let total =
        proxy_db::get_logs_count_filtered(&params.filter, params.errors_only).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse { error: e }),
            )
        })?;

    let logs =
        proxy_db::get_logs_filtered(&params.filter, params.errors_only, limit, params.offset)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse { error: e }),
                )
            })?;

    Ok(Json(LogsResponse { total, logs }))
}

// ============================================================================
// Server
// ============================================================================

/// Hostnames a local script legitimately uses to reach this server.
fn is_loopback_host(authority: &str) -> bool {
    let authority = authority.trim();
    let host = match authority.strip_prefix('[') {
        // "[::1]:19527"
        Some(rest) => rest.split(']').next().unwrap_or(""),
        // "127.0.0.1:19527" / "localhost"
        None => authority.split(':').next().unwrap_or(""),
    };
    host == "127.0.0.1" || host == "::1" || host.eq_ignore_ascii_case("localhost")
}

/// Only plain local scripts may call this API, never a web page.
///
/// The API has no credentials, so without this any website open in the maintainer's browser
/// could read the account list or switch accounts (cross-site requests, DNS rebinding):
/// - browsers attach an `Origin` header to cross-origin requests; scripts (curl, Python) do not,
/// - a rebound hostname shows up as a non-loopback `Host`.
fn is_local_caller(headers: &HeaderMap, uri_authority: Option<&str>) -> bool {
    if headers.contains_key(header::ORIGIN) {
        return false;
    }
    let host = headers
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .or(uri_authority);
    match host {
        Some(host) => is_loopback_host(host),
        None => false,
    }
}

async fn require_local_caller(req: Request, next: Next) -> Response {
    let authority = req.uri().authority().map(|a| a.as_str().to_string());
    if is_local_caller(req.headers(), authority.as_deref()) {
        next.run(req).await
    } else {
        (
            StatusCode::FORBIDDEN,
            Json(ErrorResponse {
                error:
                    "Only local scripts may call this API (browser or non-loopback Host refused)"
                        .to_string(),
            }),
        )
            .into_response()
    }
}

fn build_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/accounts", get(list_accounts))
        .route("/accounts/current", get(get_current_account))
        .route("/accounts/switch", post(switch_account))
        .route("/accounts/refresh", post(refresh_all_quotas))
        .route("/accounts/:id/bind-device", post(bind_device))
        .route("/logs", get(get_logs))
        .layer(middleware::from_fn(require_local_caller))
        .with_state(state)
}

/// Start HTTP API server
pub async fn start_server(
    port: u16,
    integration: crate::modules::integration::SystemManager,
) -> Result<(), String> {
    let app = build_router(ApiState::new(integration));

    let addr = format!("127.0.0.1:{}", port);
    logger::log_info(&format!("[HTTP API] Starting server: http://{}", addr));

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| format!("failed_to_bind_port: {}", e))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| format!("failed_to_run_server: {}", e))?;

    Ok(())
}

/// Start HTTP API server in background (non-blocking)
pub fn spawn_server(port: u16, integration: crate::modules::integration::SystemManager) {
    // Use tauri::async_runtime::spawn to ensure running within Tauri's runtime
    tauri::async_runtime::spawn(async move {
        if let Err(e) = start_server(port, integration).await {
            logger::log_error(&format!("[HTTP API] Failed to start server: {}", e));
        }
    });
}

/// App-startup hook: start the server only when the settings turn it on.
/// Settings that cannot be read leave it off (fail closed).
pub fn spawn_server_if_enabled(integration: crate::modules::integration::SystemManager) {
    match load_settings() {
        Ok(settings) => match server_port_if_enabled(&settings) {
            Some(port) => spawn_server(port, integration),
            None if settings.enabled => logger::log_error(&format!(
                "[HTTP API] Enabled but not started: {}",
                settings.validate().err().unwrap_or_default()
            )),
            None => {
                logger::log_info("[HTTP API] Disabled (default); local account API not started")
            }
        },
        Err(e) => logger::log_error(&format!(
            "[HTTP API] Not started, settings unreadable: {}",
            e
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use tower::ServiceExt;

    fn settings_file(dir: &tempfile::TempDir) -> std::path::PathBuf {
        dir.path().join(SETTINGS_FILE)
    }

    #[test]
    fn task_missing_settings_file_means_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let loaded = load_settings_from(&settings_file(&dir)).unwrap();
        assert!(!loaded.enabled, "no settings file must mean the API is off");
        assert_eq!(loaded.port, DEFAULT_PORT);
        assert!(!HttpApiSettings::default().enabled);
    }

    #[test]
    fn task_settings_file_without_enabled_key_means_disabled() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(settings_file(&dir), r#"{"port": 20000}"#).unwrap();
        let loaded = load_settings_from(&settings_file(&dir)).unwrap();
        assert!(!loaded.enabled);
        assert_eq!(loaded.port, 20000);
    }

    #[test]
    fn task_saved_settings_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let wanted = HttpApiSettings {
            enabled: true,
            port: 23456,
        };
        save_settings_to(&settings_file(&dir), &wanted).unwrap();
        assert_eq!(load_settings_from(&settings_file(&dir)).unwrap(), wanted);
    }

    #[test]
    fn task_unreadable_settings_file_is_an_error_not_defaults() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(settings_file(&dir), "{ not json").unwrap();
        assert!(load_settings_from(&settings_file(&dir)).is_err());
    }

    #[test]
    fn task_disabled_settings_do_not_start_server() {
        assert_eq!(server_port_if_enabled(&HttpApiSettings::default()), None);
        let off = HttpApiSettings {
            enabled: false,
            port: 23456,
        };
        assert_eq!(server_port_if_enabled(&off), None);
        let on = HttpApiSettings {
            enabled: true,
            port: 23456,
        };
        assert_eq!(server_port_if_enabled(&on), Some(23456));
    }

    #[test]
    fn task_enabled_with_unbindable_port_does_not_start_server() {
        let bad = HttpApiSettings {
            enabled: true,
            port: 80,
        };
        assert_eq!(server_port_if_enabled(&bad), None);
        let dir = tempfile::tempdir().unwrap();
        assert!(save_settings_to(&settings_file(&dir), &bad).is_err());
        assert!(
            !settings_file(&dir).exists(),
            "invalid settings must not be written"
        );
    }

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (k, v) in pairs {
            map.insert(
                header::HeaderName::from_bytes(k.as_bytes()).unwrap(),
                header::HeaderValue::from_str(v).unwrap(),
            );
        }
        map
    }

    #[tokio::test]
    async fn task_only_one_switch_may_run_at_a_time() {
        let slot = RwLock::new(false);
        assert!(try_begin_switch(&slot).await, "first switch must start");
        assert!(
            !try_begin_switch(&slot).await,
            "second switch while one runs must be refused (HTTP 409)"
        );
        *slot.write().await = false;
        assert!(
            try_begin_switch(&slot).await,
            "slot is reusable after the switch ends"
        );
    }

    #[test]
    fn task_local_script_with_loopback_host_is_allowed() {
        for host in [
            "127.0.0.1:19527",
            "localhost:19527",
            "LOCALHOST",
            "[::1]:19527",
        ] {
            assert!(
                is_local_caller(&headers(&[("host", host)]), None),
                "{host} must be allowed"
            );
        }
    }

    #[test]
    fn task_browser_origin_or_foreign_host_is_refused() {
        // Cross-site fetch from a web page.
        assert!(!is_local_caller(
            &headers(&[
                ("host", "127.0.0.1:19527"),
                ("origin", "https://evil.example")
            ]),
            None
        ));
        // DNS rebinding: attacker hostname resolved to 127.0.0.1.
        assert!(!is_local_caller(
            &headers(&[("host", "rebind.evil.example:19527")]),
            None
        ));
        assert!(!is_local_caller(
            &headers(&[("host", "127.0.0.1.evil.example")]),
            None
        ));
        // No host information at all is not a well-formed local call.
        assert!(!is_local_caller(&HeaderMap::new(), None));
    }

    fn request(method: &str, uri: &str, extra: &[(&str, &str)]) -> axum::http::Request<Body> {
        let mut builder = axum::http::Request::builder()
            .method(method)
            .uri(uri)
            .header("host", "127.0.0.1:19527");
        for (k, v) in extra {
            builder = builder.header(*k, *v);
        }
        builder.body(Body::empty()).unwrap()
    }

    fn router() -> Router {
        build_router(ApiState::new(
            crate::modules::integration::SystemManager::Headless,
        ))
    }

    #[tokio::test]
    async fn task_router_serves_local_scripts_and_refuses_browser_requests() {
        let ok = router()
            .oneshot(request("GET", "/health", &[]))
            .await
            .unwrap();
        assert_eq!(ok.status(), StatusCode::OK);

        let browser = router()
            .oneshot(request(
                "GET",
                "/health",
                &[("origin", "https://evil.example")],
            ))
            .await
            .unwrap();
        assert_eq!(browser.status(), StatusCode::FORBIDDEN);

        // CORS preflight must not be answered with permissive headers any more.
        let preflight = router()
            .oneshot(request(
                "OPTIONS",
                "/accounts/switch",
                &[
                    ("origin", "https://evil.example"),
                    ("access-control-request-method", "POST"),
                ],
            ))
            .await
            .unwrap();
        assert_eq!(preflight.status(), StatusCode::FORBIDDEN);
        assert!(preflight
            .headers()
            .get("access-control-allow-origin")
            .is_none());
    }

    #[tokio::test]
    async fn task_bind_device_route_matches_real_account_ids() {
        // axum 0.7 path parameters are `:id`; `{id}` would be a literal segment and 404.
        // No JSON content-type, so the extractor rejects the request before the handler
        // touches any account data.
        let resp = router()
            .oneshot(request(
                "POST",
                "/accounts/some-account-id/bind-device",
                &[],
            ))
            .await
            .unwrap();
        assert_ne!(resp.status(), StatusCode::NOT_FOUND);
    }
}
