use crate::proxy::server::AppState;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

pub async fn service_status_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let running = {
        let r = state.is_running.read().await;
        *r
    };

    service_status_middleware_with_running(running, request, next).await
}

pub(crate) async fn service_status_middleware_with_running(
    running: bool,
    request: Request,
    next: Next,
) -> Response {
    if let Err(status) = crate::proxy::middleware::internal::require_internal_loopback(&request) {
        return status.into_response();
    }

    let path = request.uri().path();

    // Always allow Admin API, verified internal endpoints and Auth callback
    if path.starts_with("/api/")
        || path.starts_with("/internal/")
        || path == "/auth/callback"
        || path == "/health"
    {
        return next.run(request).await;
    }

    if !running {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "Proxy service is currently disabled".to_string(),
        )
            .into_response();
    }

    next.run(request).await
}
