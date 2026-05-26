//! Custom Transport implementation example.
//!
//! Demonstrates implementing the simplest Transport trait using an in-memory queue.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use pawbun_mcp_core::protocol::{JsonRpcRequest, JsonRpcResponse};
use pawbun_mcp_core::transport::{Transport, TransportError};

#[derive(Debug)]
struct InMemoryTransport {
    responses: Arc<Mutex<VecDeque<JsonRpcResponse>>>,
}

impl InMemoryTransport {
    fn new(responses: Vec<JsonRpcResponse>) -> Self {
        Self {
            responses: Arc::new(Mutex::new(responses.into_iter().collect())),
        }
    }
}

impl Transport for InMemoryTransport {
    fn request(&mut self, _req: JsonRpcRequest) -> Result<JsonRpcResponse, TransportError> {
        let mut guard = self.responses.lock().unwrap();
        guard.pop_front().ok_or(TransportError::UnexpectedEof)
    }

    fn close(self: Box<Self>) -> Result<(), TransportError> {
        Ok(())
    }
}

fn main() {
    let responses = vec![
        JsonRpcResponse::ok(Some(1i64.into()), serde_json::json!("hello")),
    ];

    let mut transport = InMemoryTransport::new(responses);
    let req = JsonRpcRequest::new(1i64, "test", None);
    let resp = transport.request(req).unwrap();
    println!("Response: {:?}", resp.result);
}
