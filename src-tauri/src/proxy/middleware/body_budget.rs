// Request-size limits shared by the HTTP and WebSocket paths, and a budget for the request
// bodies being processed at the same time.
//
// A request body is held in memory several times over while it is handled (raw bytes, the
// parsed JSON tree, the typed request, the upstream payload), so the per-request size limit
// alone does not bound memory: N clients sending the largest allowed body at once multiply
// it by N, and the proxy shares its process with the desktop app.
use axum::{
    extract::{ws::WebSocketUpgrade, Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Default request body limit: 100 MB.
pub const DEFAULT_MAX_BODY_BYTES: usize = 100 * 1024 * 1024;
/// Bodies below this size are never queued.
pub const DEFAULT_FREE_BELOW_BYTES: usize = 1024 * 1024;
/// How long a large request may wait for budget before it is turned away.
pub const DEFAULT_MAX_WAIT: Duration = Duration::from_secs(60);

/// Largest accepted request body, in bytes: `ABV_MAX_BODY_SIZE`, default 100 MB.
/// One WebSocket message is one request, so the same number limits a message and a frame.
pub fn max_request_body_bytes() -> usize {
    std::env::var("ABV_MAX_BODY_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_MAX_BODY_BYTES)
}

/// Apply the request size limit to a WebSocket upgrade.
///
/// Without this, tungstenite's own defaults apply (64 MiB per message, 16 MiB per frame),
/// unrelated to the configured HTTP limit, and `DefaultBodyLimit` does not cover messages.
pub fn limit_websocket_messages(ws: WebSocketUpgrade, max_bytes: usize) -> WebSocketUpgrade {
    ws.max_message_size(max_bytes).max_frame_size(max_bytes)
}

// Budget accounting granularity. `Semaphore::acquire_many` takes a `u32`.
const UNIT_BYTES: usize = 64 * 1024;

/// Total declared size of the large request bodies that may be in flight at once.
pub struct BodyBudget {
    semaphore: Arc<Semaphore>,
    total_units: u32,
    free_below_bytes: usize,
    /// Charged for a body of unknown length (`Transfer-Encoding: chunked`).
    unknown_length_bytes: usize,
    max_wait: Duration,
}

impl BodyBudget {
    pub fn new(
        total_bytes: usize,
        free_below_bytes: usize,
        unknown_length_bytes: usize,
        max_wait: Duration,
    ) -> Self {
        let total_units = Self::units(total_bytes).max(1);
        Self {
            semaphore: Arc::new(Semaphore::new(total_units as usize)),
            total_units,
            free_below_bytes,
            unknown_length_bytes,
            max_wait,
        }
    }

    /// Budget for a server whose per-request limit is `max_body_bytes`.
    ///
    /// `ABV_MAX_INFLIGHT_BODY_BYTES` sets the total; the default equals the per-request
    /// limit, i.e. one maximum-size body at a time, or many smaller ones adding up to it.
    pub fn from_env(max_body_bytes: usize) -> Self {
        let total = std::env::var("ABV_MAX_INFLIGHT_BODY_BYTES")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|n| *n > 0)
            .unwrap_or(max_body_bytes);
        // A chunked body does not say how large it is. Charging it the full limit would make
        // a client that always uploads chunked run one request at a time, so it is charged a
        // quarter: at most four such bodies at once, each still capped by the per-request limit.
        Self::new(
            total,
            DEFAULT_FREE_BELOW_BYTES,
            max_body_bytes / 4,
            DEFAULT_MAX_WAIT,
        )
    }

    fn units(bytes: usize) -> u32 {
        u32::try_from(bytes.div_ceil(UNIT_BYTES)).unwrap_or(u32::MAX)
    }

    /// Budget units a request must hold; `0` means it is not counted.
    /// `declared_len` is `None` for a body of unknown length.
    fn charge(&self, has_body: bool, declared_len: Option<u64>) -> u32 {
        if !has_body {
            return 0;
        }
        let bytes = match declared_len {
            Some(len) => usize::try_from(len).unwrap_or(usize::MAX),
            None => self.unknown_length_bytes,
        };
        if bytes < self.free_below_bytes {
            return 0;
        }
        // A body larger than the whole budget runs alone instead of never running; the
        // per-request limit still decides whether it is accepted at all.
        Self::units(bytes).min(self.total_units)
    }

    /// Wait for `units` of budget. `Err` when none became available in time.
    async fn acquire(&self, units: u32) -> Result<Option<OwnedSemaphorePermit>, ()> {
        if units == 0 {
            return Ok(None);
        }
        match tokio::time::timeout(
            self.max_wait,
            self.semaphore.clone().acquire_many_owned(units),
        )
        .await
        {
            Ok(Ok(permit)) => Ok(Some(permit)),
            _ => Err(()),
        }
    }
}

/// Hold budget for a large request body from before it is read until the handler has
/// produced its response.
///
/// Must sit outside every layer that reads the body (the monitor does), so a queued request
/// costs a socket, not a buffered body. Small requests pass straight through.
pub async fn body_budget_middleware(
    State(budget): State<Arc<BodyBudget>>,
    request: Request,
    next: Next,
) -> Response {
    let declared_len = request
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.trim().parse::<u64>().ok());
    let has_body =
        declared_len.is_some() || request.headers().contains_key(header::TRANSFER_ENCODING);
    let units = budget.charge(has_body, declared_len);

    let _permit = match budget.acquire(units).await {
        Ok(permit) => permit,
        Err(()) => {
            tracing::warn!(
                "Large request turned away: no body budget within {:?} ({} {}, declared {:?} bytes)",
                budget.max_wait,
                request.method(),
                request.uri().path(),
                declared_len
            );
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                [(header::RETRY_AFTER, "5")],
                Json(serde_json::json!({
                    "error": {
                        "message": "The proxy is busy with other large requests. Retry shortly.",
                        "type": "server_busy",
                        "code": "large_request_budget_exhausted"
                    }
                })),
            )
                .into_response();
        }
    };
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::extract::ws::{Message, WebSocket};
    use axum::http::Request as HttpRequest;
    use axum::routing::{get, post};
    use axum::Router;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tower::ServiceExt;

    const MB: usize = 1024 * 1024;

    #[test]
    fn task_budget_charges_only_large_or_unknown_length_bodies() {
        let budget = BodyBudget::new(100 * MB, MB, 100 * MB, DEFAULT_MAX_WAIT);
        // No body, or a small one: never queued.
        assert_eq!(budget.charge(false, None), 0);
        assert_eq!(budget.charge(true, Some(0)), 0);
        assert_eq!(budget.charge(true, Some((MB - 1) as u64)), 0);
        // Large: proportional to the declared size.
        assert_eq!(budget.charge(true, Some(MB as u64)), 16);
        assert_eq!(budget.charge(true, Some((10 * MB) as u64)), 160);
        // Unknown length (chunked) is charged the configured stand-in size.
        assert_eq!(budget.charge(true, None), budget.total_units);
        let chunked = BodyBudget::from_env(100 * MB);
        assert_eq!(chunked.charge(true, None), 400);
        // Larger than the whole budget (or an absurd Content-Length): runs alone.
        assert_eq!(budget.charge(true, Some(u64::MAX)), budget.total_units);
    }

    /// Router whose handler reports how many declared bytes were in flight at once.
    fn tracking_app(
        budget: Arc<BodyBudget>,
        in_flight: Arc<AtomicUsize>,
        peak: Arc<AtomicUsize>,
    ) -> Router {
        Router::new()
            .route(
                "/v1/messages",
                post(move |request: Request| {
                    let in_flight = in_flight.clone();
                    let peak = peak.clone();
                    async move {
                        let declared: usize = request
                            .headers()
                            .get(header::CONTENT_LENGTH)
                            .and_then(|v| v.to_str().ok())
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0);
                        let now = in_flight.fetch_add(declared, Ordering::SeqCst) + declared;
                        peak.fetch_max(now, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(40)).await;
                        in_flight.fetch_sub(declared, Ordering::SeqCst);
                        "ok"
                    }
                }),
            )
            .layer(axum::middleware::from_fn_with_state(
                budget,
                body_budget_middleware,
            ))
    }

    fn declared_post(len: usize) -> HttpRequest<Body> {
        // Only the declared length matters to the budget; the handler does not read the body.
        HttpRequest::builder()
            .method("POST")
            .uri("/v1/messages")
            .header(header::CONTENT_LENGTH, len.to_string())
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn task_concurrent_large_bodies_stay_within_the_budget() {
        let in_flight = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let budget = Arc::new(BodyBudget::new(20 * MB, MB, 20 * MB, DEFAULT_MAX_WAIT));
        let app = tracking_app(budget, in_flight, peak.clone());

        // 8 x 8 MB at once against a 20 MB budget: at most two may run together.
        let mut tasks = Vec::new();
        for _ in 0..8 {
            let app = app.clone();
            tasks.push(tokio::spawn(async move {
                app.oneshot(declared_post(8 * MB)).await.unwrap().status()
            }));
        }
        for task in tasks {
            assert_eq!(task.await.unwrap(), StatusCode::OK);
        }
        let peak = peak.load(Ordering::SeqCst);
        assert!(
            peak <= 20 * MB,
            "peak in-flight {peak} bytes exceeds budget"
        );
        assert!(peak >= 8 * MB);
    }

    #[tokio::test]
    async fn task_without_a_budget_the_same_load_runs_all_at_once() {
        // Control for the test above: the bound comes from the budget, not from the test.
        let in_flight = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let budget = Arc::new(BodyBudget::new(1024 * MB, MB, 20 * MB, DEFAULT_MAX_WAIT));
        let app = tracking_app(budget, in_flight, peak.clone());

        let mut tasks = Vec::new();
        for _ in 0..8 {
            let app = app.clone();
            tasks.push(tokio::spawn(async move {
                app.oneshot(declared_post(8 * MB)).await.unwrap().status()
            }));
        }
        for task in tasks {
            assert_eq!(task.await.unwrap(), StatusCode::OK);
        }
        assert_eq!(peak.load(Ordering::SeqCst), 64 * MB);
    }

    #[tokio::test]
    async fn task_small_requests_are_not_queued_behind_large_ones() {
        let budget = Arc::new(BodyBudget::new(
            8 * MB,
            MB,
            8 * MB,
            Duration::from_millis(50),
        ));
        // Something large holds the whole budget.
        let held = budget.acquire(budget.total_units).await.unwrap();
        let app = tracking_app(
            budget.clone(),
            Arc::new(AtomicUsize::new(0)),
            Arc::new(AtomicUsize::new(0)),
        );

        // An ordinary request still goes through at once.
        let status = app
            .clone()
            .oneshot(declared_post(4096))
            .await
            .unwrap()
            .status();
        assert_eq!(status, StatusCode::OK);

        // Another large one waits, then is told to retry instead of hanging forever.
        let response = app.clone().oneshot(declared_post(4 * MB)).await.unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(response.headers().contains_key(header::RETRY_AFTER));

        // Once the budget is released, large requests run again.
        drop(held);
        let status = app.oneshot(declared_post(4 * MB)).await.unwrap().status();
        assert_eq!(status, StatusCode::OK);
    }

    /// Client-to-server WebSocket frame header (masked, FIN, text) declaring `len` bytes.
    fn text_frame_header(len: u64) -> Vec<u8> {
        let mut frame = vec![0x81u8];
        if len < 126 {
            frame.push(0x80 | len as u8);
        } else if len <= u16::MAX as u64 {
            frame.push(0x80 | 126);
            frame.extend_from_slice(&(len as u16).to_be_bytes());
        } else {
            frame.push(0x80 | 127);
            frame.extend_from_slice(&len.to_be_bytes());
        }
        frame.extend_from_slice(&[0, 0, 0, 0]); // zero mask: payload goes unchanged
        frame
    }

    /// Open a WebSocket to a server limited to `limit` bytes, send `frames`, and return what
    /// the server-side session saw: `Ok(text)` per message, `Err(())` for a read error.
    /// Stops after `expected` events or when nothing more arrives.
    async fn ws_session_events(
        limit: usize,
        frames: Vec<u8>,
        expected: usize,
    ) -> Vec<Result<String, ()>> {
        let (events_tx, mut events_rx) = tokio::sync::mpsc::unbounded_channel();
        let app = Router::new().route(
            "/v1/responses",
            get(move |ws: WebSocketUpgrade| {
                let events_tx = events_tx.clone();
                async move {
                    limit_websocket_messages(ws, limit).on_upgrade(
                        move |mut socket: WebSocket| async move {
                            while let Some(message) = socket.recv().await {
                                match message {
                                    Ok(Message::Text(text)) => {
                                        let _ = events_tx.send(Ok(text));
                                    }
                                    Ok(_) => {}
                                    Err(_) => {
                                        let _ = events_tx.send(Err(()));
                                        break;
                                    }
                                }
                            }
                        },
                    )
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
        let handshake = format!(
            "GET /v1/responses HTTP/1.1\r\nHost: {addr}\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n"
        );
        stream.write_all(handshake.as_bytes()).await.unwrap();
        let mut received = Vec::new();
        let mut buf = [0u8; 512];
        while !received.windows(4).any(|w| w == b"\r\n\r\n") {
            let n = stream.read(&mut buf).await.unwrap();
            assert!(n > 0, "server closed during the handshake");
            received.extend_from_slice(&buf[..n]);
        }
        assert!(received.starts_with(b"HTTP/1.1 101"));

        stream.write_all(&frames).await.unwrap();
        stream.flush().await.unwrap();

        let mut events = Vec::new();
        while events.len() < expected {
            match tokio::time::timeout(Duration::from_secs(20), events_rx.recv()).await {
                Ok(Some(event)) => events.push(event),
                _ => break,
            }
        }
        server.abort();
        events
    }

    #[tokio::test]
    async fn task_websocket_message_within_the_limit_is_delivered() {
        let mut frames = text_frame_header(5);
        frames.extend_from_slice(b"hello");
        assert_eq!(
            ws_session_events(1024, frames, 1).await,
            vec![Ok("hello".to_string())]
        );
    }

    #[tokio::test]
    async fn task_websocket_frame_over_the_limit_ends_the_session_before_it_is_buffered() {
        // Only the header is sent: the declared length alone must be refused, so the
        // oversized payload is never accumulated.
        let frames = text_frame_header(1025);
        assert_eq!(ws_session_events(1024, frames, 1).await, vec![Err(())]);

        // tungstenite's default frame limit is 16 MiB; a 20 MiB frame only passes because
        // the configured limit says so, i.e. the limit really is the one being applied.
        let mut frames = text_frame_header(20 * MB as u64);
        frames.resize(frames.len() + 20 * MB, b'a');
        let events = ws_session_events(32 * MB, frames, 1).await;
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].as_ref().map(String::len), Ok(20 * MB));
    }
}
