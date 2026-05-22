//! SSE (Server-Sent Events) server transport.
//!
//! Implements the MCP SSE transport specification:
//! 1. Client connects to `GET /sse` and receives an `endpoint` event
//!    with the POST URL for JSON-RPC requests.
//! 2. Client sends JSON-RPC requests via `POST /message?sessionId=xxx`.
//! 3. Server routes responses back through that session's SSE stream.

use std::collections::HashMap;
use std::io::ErrorKind;
use std::sync::Arc;

use pawbun_mcp_core::protocol::{JsonRpcRequest, JsonRpcResponse};
use pawbun_mcp_core::transport::{ServerTransport, TransportError};
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, oneshot, RwLock};

use axum::{
    extract::{Query, State},
    response::sse::{Event, Sse},
    routing::{get, post},
    Router,
};
use futures::stream::Stream;
use std::convert::Infallible;
use std::time::Duration;

/// A request paired with its response channel, tagged by session.
struct TaggedRequest {
    #[allow(dead_code)]
    session_id: String,
    request: JsonRpcRequest,
    response_tx: oneshot::Sender<JsonRpcResponse>,
}

/// SSE server transport.
pub struct SseServerTransport {
    /// Tagged requests from POST handler.
    request_rx: mpsc::UnboundedReceiver<TaggedRequest>,
    /// Response channel for the currently-being-handled request.
    current_response_tx: Option<oneshot::Sender<JsonRpcResponse>>,
    /// Tokio runtime that owns the axum server.
    runtime: Runtime,
}

#[derive(Debug)]
struct AppState {
    /// Channel to send tagged requests from POST handler to recv().
    request_tx: mpsc::UnboundedSender<TaggedRequest>,
    /// Per-session SSE response channels, keyed by session ID.
    sessions: RwLock<HashMap<String, mpsc::UnboundedSender<JsonRpcResponse>>>,
}

impl SseServerTransport {
    /// Creates a new SSE transport and starts the axum server in a background task.
    pub fn new(bind_addr: &str) -> Result<Self, String> {
        let bind_addr = bind_addr.to_string();

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|e| format!("failed to create tokio runtime: {e}"))?;

        let (request_tx, request_rx) = mpsc::unbounded_channel();
        let state = Arc::new(AppState {
            request_tx,
            sessions: RwLock::new(HashMap::new()),
        });

        let app_state = state.clone();
        let addr = bind_addr.clone();

        runtime.spawn(async move {
            let app = Router::new()
                .route("/sse", get(sse_handler))
                .route("/message", post(message_handler))
                .with_state(app_state);

            let listener = match tokio::net::TcpListener::bind(&addr).await {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("SSE transport bind failed: {e}");
                    return;
                }
            };

            if let Err(e) = axum::serve(listener, app).await {
                eprintln!("SSE server error: {e}");
            }
        });

        Ok(Self {
            request_rx,
            current_response_tx: None,
            runtime,
        })
    }
}

impl ServerTransport for SseServerTransport {
    fn recv(&mut self) -> Result<JsonRpcRequest, TransportError> {
        self.runtime.block_on(async {
            match self.request_rx.recv().await {
                Some(tagged) => {
                    self.current_response_tx = Some(tagged.response_tx);
                    Ok(tagged.request)
                }
                None => Err(TransportError::UnexpectedEof),
            }
        })
    }

    fn send(&mut self, resp: JsonRpcResponse) -> Result<(), TransportError> {
        // Notification responses (empty) are suppressed.
        let is_empty_notification =
            resp.id.is_none() && resp.result.is_none() && resp.error.is_none();
        if is_empty_notification {
            return Ok(());
        }

        if let Some(tx) = self.current_response_tx.take() {
            // Route the response to the SSE session via the stored oneshot channel.
            let _ = tx.send(resp);
            Ok(())
        } else {
            Err(TransportError::Io {
                message: "no pending response channel for SSE send".into(),
                kind: ErrorKind::Other,
            })
        }
    }

    fn close(self: Box<Self>) -> Result<(), TransportError> {
        self.runtime.shutdown_timeout(Duration::from_secs(5));
        Ok(())
    }
}

// ── Axum handlers ──

#[derive(serde::Deserialize)]
struct MessageQuery {
    #[serde(rename = "sessionId")]
    session_id: String,
}

async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let session_id = uuid::Uuid::new_v4().to_string();
    let (tx, mut rx) = mpsc::unbounded_channel();

    state.sessions.write().await.insert(session_id.clone(), tx);

    // Spawn a task that converts channel responses into SSE events
    let stream = async_stream::stream! {
        // First event: tell client where to POST
        yield Ok(Event::default()
            .event("endpoint")
            .data(format!("/message?sessionId={}", session_id)));

        // Then stream responses as they arrive
        while let Some(resp) = rx.recv().await {
            let data = serde_json::to_string(&resp).unwrap_or_default();
            yield Ok(Event::default().event("message").data(data));
        }

        // Clean up session when channel closes
        state.sessions.write().await.remove(&session_id);
    };

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("ping"),
    )
}

async fn message_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<MessageQuery>,
    body: String,
) -> Result<String, (axum::http::StatusCode, String)> {
    let req: JsonRpcRequest = serde_json::from_str(&body)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    let is_notification = req.id.is_none();

    if is_notification {
        // Notifications: forward to the request channel for handler processing.
        // No response expected — create a dummy channel that gets dropped.
        let (tx, _rx) = oneshot::channel();
        state
            .request_tx
            .send(TaggedRequest {
                session_id: query.session_id,
                request: req,
                response_tx: tx,
            })
            .map_err(|e| {
                (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
            })?;
        return Ok("Accepted".into());
    }

    // For requests with an id: create a oneshot channel, send to request queue,
    // then wait for the handler to produce a response and route it to the
    // session's SSE channel.
    let (response_tx, response_rx) = oneshot::channel();

    // Clone before moving query.session_id into TaggedRequest.
    let session_id_for_spawn = query.session_id.clone();

    // Send the tagged request to the transport's recv() queue.
    state
        .request_tx
        .send(TaggedRequest {
            session_id: query.session_id,
            request: req,
            response_tx,
        })
        .map_err(|e| {
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        })?;

    // Wait for the handler to produce a response (via oneshot from send()),
    // then forward it to the SSE session channel.
    let state_for_response = state.clone();
    tokio::spawn(async move {
        match response_rx.await {
            Ok(resp) => {
                let sessions_guard = state_for_response.sessions.read().await;
                if let Some(sse_tx) = sessions_guard.get(&session_id_for_spawn) {
                    let _ = sse_tx.send(resp);
                }
            }
            Err(_) => {
                // Response channel closed — handler may have shut down.
            }
        }
    });

    // Return 202 Accepted — the actual response comes through SSE.
    Ok("Accepted".into())
}
