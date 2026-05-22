//! SSE transport placeholder — implemented in Phase 3.
//!
//! When the `http` feature is enabled, this module provides a full
//! SSE-based MCP transport backed by tokio + axum.

use pawbun_mcp_core::protocol::{JsonRpcRequest, JsonRpcResponse};
use pawbun_mcp_core::transport::{ServerTransport, TransportError};

use std::io::ErrorKind;

/// SSE server transport.
///
/// Fully implemented only when the `http` feature is enabled.
pub struct SseServerTransport {
    #[allow(dead_code)]
    bind_addr: String,
}

impl SseServerTransport {
    /// Creates a new SSE transport.
    #[cfg(feature = "http")]
    pub fn new(bind_addr: &str) -> Result<Self, String> {
        // TODO: Phase 3 — start tokio runtime + axum server
        Err("SSE transport not yet implemented".into())
    }

    /// Stub for when http feature is disabled.
    #[cfg(not(feature = "http"))]
    pub fn new(bind_addr: &str) -> Result<Self, String> {
        Ok(Self {
            bind_addr: bind_addr.to_string(),
        })
    }
}

impl ServerTransport for SseServerTransport {
    fn recv(&mut self) -> Result<JsonRpcRequest, TransportError> {
        Err(TransportError::Io {
            message: "SSE transport not implemented".into(),
            kind: ErrorKind::Other,
        })
    }

    fn send(&mut self, _resp: JsonRpcResponse) -> Result<(), TransportError> {
        Err(TransportError::Io {
            message: "SSE transport not implemented".into(),
            kind: ErrorKind::Other,
        })
    }

    fn close(self: Box<Self>) -> Result<(), TransportError> {
        Ok(())
    }
}
