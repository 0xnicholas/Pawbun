//! SSE transport placeholder — implemented in Task 3.1.

pub struct SseServerTransport {}

impl SseServerTransport {
    pub fn new(_bind_addr: &str) -> Result<Self, String> {
        Err("SSE transport not yet implemented".into())
    }
}
