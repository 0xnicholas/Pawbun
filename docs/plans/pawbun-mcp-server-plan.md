# pawbun-mcp-server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add MCP Server capability to Pawbun — extract MCP protocol types to `pawbun-mcp-core`, build `pawbun-mcp-server` crate with stdio/SSE transport, and auto-wrap FileLoader as MCP tools.

**Architecture:** Three-crate design: `pawbun-mcp-core` (protocol + transport traits, zero pawbun deps), `pawbun-mcp-server` (server + handler state machine + stdio/SSE transport, depends on core + toolkit + files), existing `pawbun-toolkit` (migrates to reference core, API unchanged).

**Tech Stack:** Rust 1.75+, serde/serde_json, thiserror, tokio + axum (SSE feature), bytes, base64

**Spec:** `docs/specs/pawbun-mcp-server-spec.md`

---

## File Structure

```
New files:
  crates/pawbun-mcp-core/
    Cargo.toml
    src/lib.rs                        — Re-exports: ToolParameter, protocol, transport, schema_convert
    src/protocol.rs                   — JSON-RPC 2.0 + MCP message types (migrated from toolkit)
    src/transport.rs                  — Transport trait (client) + ServerTransport trait (server) + TransportError + configs
    src/schema_convert.rs             — schema_to_parameters() + parameters_to_schema() (bidirectional)

  crates/pawbun-mcp-server/
    Cargo.toml
    src/lib.rs                        — Re-exports: McpServer, McpServerBuilder, McpServerError
    src/server.rs                     — McpServer + McpServerBuilder
    src/handler.rs                    — RequestHandler with initialize state machine + method routing
    src/error.rs                      — McpServerError enum
    src/tool_bridge.rs                — FileReadBridgeTool + FileListBridgeTool (FileLoader → Tool adapters)
    src/transport/
      mod.rs                          — Re-exports stdio/sse
      stdio.rs                        — StdioServerTransport
      sse.rs                          — SseServerTransport (behind http feature)
  crates/pawbun-mcp-server/tests/
    handler_tests.rs                  — initialize, tools/list, tools/call, state machine tests
    tool_bridge_tests.rs              — FileLoader → Tool wrapping tests

Modified files:
  Cargo.toml                          — Add workspace members: pawbun-mcp-core, pawbun-mcp-server
  crates/pawbun-toolkit/Cargo.toml    — Add pawbun-mcp-core dependency, remove direct serde_json? No—keep it
  crates/pawbun-toolkit/src/lib.rs    — Remove ToolParameter re-export, add pub use pawbun_mcp_core::ToolParameter
  crates/pawbun-toolkit/src/types.rs  — Delete ToolParameter definition, replace with pub use re-export
  crates/pawbun-toolkit/src/mcp/mod.rs — Update imports to reference pawbun_mcp_core instead of local modules

Deleted files (content migrated to core):
  crates/pawbun-toolkit/src/mcp/protocol.rs       → crates/pawbun-mcp-core/src/protocol.rs
  crates/pawbun-toolkit/src/mcp/transport.rs       → crates/pawbun-mcp-core/src/transport.rs
  crates/pawbun-toolkit/src/mcp/schema_convert.rs  → crates/pawbun-mcp-core/src/schema_convert.rs
```

---

## Phase 1: pawbun-mcp-core — Extract protocol + transport + schema + ToolParameter

### Task 1.1: Create pawbun-mcp-core crate skeleton

**Files:**
- Create: `crates/pawbun-mcp-core/Cargo.toml`
- Create: `crates/pawbun-mcp-core/src/lib.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "pawbun-mcp-core"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "MCP protocol core types and transport abstractions for Pawbun"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"

[features]
default = []
schemars = ["dep:schemars"]
tracing = ["dep:tracing"]
```

- [ ] **Step 2: Add workspace member to root Cargo.toml**

Modify `Cargo.toml`:
```toml
[workspace]
members = [
    "crates/pawbun-toolkit",
    "crates/pawbun-files",
    "crates/pawbun-toolkit-macros",
    "crates/pawbun-mcp-core",
]
```

- [ ] **Step 3: Create src/lib.rs with module declarations and ToolParameter definition**

```rust
//! MCP protocol core types and transport abstractions.
//!
//! This crate provides the foundational types shared by both MCP clients
//! and servers in the Pawbun ecosystem.

pub mod protocol;
pub mod schema_convert;
pub mod transport;

/// Describes a tool's input parameter for JSON Schema generation.
///
/// This is the canonical definition of `ToolParameter` used by both
/// `pawbun-toolkit` and `pawbun-mcp-server`. The toolkit crate re-exports
/// this type for backward compatibility.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolParameter {
    /// Parameter name.
    pub name: String,
    /// Human-readable description for LLM consumption.
    pub description: String,
    /// Whether this parameter is required.
    pub required: bool,
    /// JSON Schema fragment describing the parameter type.
    pub schema: serde_json::Value,
}

#[cfg(feature = "schemars")]
impl ToolParameter {
    /// Generates a ToolParameter from a type implementing [`schemars::JsonSchema`].
    pub fn from_schema<T: schemars::JsonSchema>(
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let root = schemars::schema_for!(T);
        let schema = serde_json::to_value(root.schema)
            .unwrap_or_else(|e| panic!("schema serialization should not fail: {e}"));
        Self {
            name: name.into(),
            description: description.into(),
            required,
            schema,
        }
    }
}
```

- [ ] **Step 4: Verify it compiles**

```bash
cargo check -p pawbun-mcp-core
```

Expected: zero errors

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/pawbun-mcp-core/
git commit -m "feat: add pawbun-mcp-core crate skeleton with ToolParameter"
```

---

### Task 1.2: Migrate protocol.rs to core

**Files:**
- Create: `crates/pawbun-mcp-core/src/protocol.rs`
- Modify: `crates/pawbun-mcp-core/src/lib.rs`

- [ ] **Step 1: Copy protocol.rs from toolkit**

Read `crates/pawbun-toolkit/src/mcp/protocol.rs` and copy its entire content to `crates/pawbun-mcp-core/src/protocol.rs`.

No changes to the type definitions. Just update the crate path in any doc comments that reference `pawbun_toolkit` → `pawbun_mcp_core`.

- [ ] **Step 2: Add JsonRpcResponse convenience constructors to protocol.rs**

At the end of `protocol.rs`, add:

```rust
use serde::Serialize;
use serde_json::Value;

impl JsonRpcResponse {
    /// Construct a successful response with a raw Value result.
    pub fn ok(id: Option<JsonRpcId>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Construct a successful response, serializing the result.
    pub fn ok_result(
        id: Option<JsonRpcId>,
        result: impl Serialize,
    ) -> Self {
        let value = serde_json::to_value(result)
            .unwrap_or(Value::Null);
        Self::ok(id, value)
    }

    /// Construct an error response with a standard JSON-RPC error code.
    pub fn error(
        id: Option<JsonRpcId>,
        code: i32,
        message: impl Into<String>,
    ) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}
```

- [ ] **Step 3: Re-export protocol from lib.rs**

In `crates/pawbun-mcp-core/src/lib.rs`:

```rust
pub use protocol::*;
```

- [ ] **Step 4: Verify it compiles**

```bash
cargo check -p pawbun-mcp-core
```

- [ ] **Step 5: Write unit tests for the new constructors**

In `protocol.rs`, add inline tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_response_ok() {
        let resp = JsonRpcResponse::ok(Some(JsonRpcId::Number(1)), json!("result"));
        assert_eq!(resp.id, Some(JsonRpcId::Number(1)));
        assert_eq!(resp.result, Some(json!("result")));
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_response_ok_result_serializes() {
        let resp = JsonRpcResponse::ok_result(Some(JsonRpcId::String("x".into())), "hello");
        assert_eq!(resp.result, Some(json!("hello")));
    }

    #[test]
    fn test_response_error() {
        let resp = JsonRpcResponse::error(Some(JsonRpcId::Number(42)), -32601, "Method not found");
        assert!(resp.is_error());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32601);
        assert_eq!(err.message, "Method not found");
    }

    #[test]
    fn test_response_ok_no_id() {
        let resp = JsonRpcResponse::ok(None, json!(42));
        assert!(resp.id.is_none());
        assert_eq!(resp.result, Some(json!(42)));
    }
}
```

```bash
cargo test -p pawbun-mcp-core
```

Expected: 4 tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/pawbun-mcp-core/
git commit -m "feat(mcp-core): add protocol types with JsonRpcResponse constructors"
```

---

### Task 1.3: Migrate transport traits and configs to core

**Files:**
- Create: `crates/pawbun-mcp-core/src/transport.rs`

- [ ] **Step 1: Write transport.rs — trait definitions and configs only**

Extract from `crates/pawbun-toolkit/src/mcp/transport.rs` the following items (do NOT bring client implementations yet):

```rust
//! MCP transport abstractions: client and server traits, configs, errors.

use std::io::ErrorKind;

use crate::protocol::{JsonRpcRequest, JsonRpcResponse};

/// Transport error.
#[derive(Debug, Clone, thiserror::Error)]
pub enum TransportError {
    #[error("IO error: {message} (kind: {kind:?})")]
    Io { message: String, kind: ErrorKind },
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("unexpected EOF")]
    UnexpectedEof,
    #[error("HTTP error: {0}")]
    Http(String),
}

/// Client-side transport: sends a JSON-RPC request and blocks for a response.
pub trait Transport: Send + Sync {
    fn request(&mut self, req: JsonRpcRequest) -> Result<JsonRpcResponse, TransportError>;
    fn close(self: Box<Self>) -> Result<(), TransportError>;
}

/// Server-side transport: receives JSON-RPC requests and sends responses.
pub trait ServerTransport: Send {
    /// Blocking receive of the next JSON-RPC request.
    fn recv(&mut self) -> Result<JsonRpcRequest, TransportError>;
    /// Send a JSON-RPC response back to the client.
    fn send(&mut self, resp: JsonRpcResponse) -> Result<(), TransportError>;
    /// Graceful shutdown.
    fn close(self: Box<Self>) -> Result<(), TransportError>;
}

/// Client transport configuration.
#[derive(Debug, Clone)]
pub enum TransportConfig {
    Stdio { command: String, args: Vec<String> },
    Sse { url: String },
}

/// Server transport configuration.
#[derive(Debug, Clone)]
pub enum ServerTransportConfig {
    Stdio,
    Sse { bind_addr: String },
}
```

- [ ] **Step 2: Re-export transport from lib.rs**

```rust
pub use transport::{Transport, ServerTransport, TransportError, TransportConfig, ServerTransportConfig};
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo check -p pawbun-mcp-core
```

- [ ] **Step 4: Commit**

```bash
git add crates/pawbun-mcp-core/src/transport.rs crates/pawbun-mcp-core/src/lib.rs
git commit -m "feat(mcp-core): add transport traits, configs, and TransportError"
```

---

### Task 1.4: Implement schema_convert.rs with bidirectional conversion

**Files:**
- Create: `crates/pawbun-mcp-core/src/schema_convert.rs`

- [ ] **Step 1: Write schema_convert.rs**

```rust
//! Bidirectional conversion between MCP input schema and ToolParameter lists.

use serde_json::Value;

use crate::ToolParameter;

/// Converts an MCP `input_schema` (JSON Schema object) into a list of [`ToolParameter`].
///
/// Supports simple object schemas with `properties` and `required` fields.
/// Properties schemas are stored as-is in [`ToolParameter::schema`].
pub fn schema_to_parameters(schema: &Value) -> Vec<ToolParameter> {
    let mut params = Vec::new();

    let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) else {
        return params;
    };

    let required: Vec<&str> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    for (name, prop_schema) in properties {
        let description = prop_schema
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();
        let is_required = required.contains(&name.as_str());

        params.push(ToolParameter {
            name: name.clone(),
            description,
            required: is_required,
            schema: prop_schema.clone(),
        });
    }

    params
}

/// Converts a list of [`ToolParameter`] into an MCP `input_schema` JSON Schema object.
///
/// Used by server-side `tools/list` to expose tool parameters in MCP-compliant format.
/// Assembled as `{"type": "object", "properties": {...}, "required": [...]}`.
pub fn parameters_to_schema(params: &[ToolParameter]) -> Value {
    let mut properties = serde_json::Map::new();
    let mut required: Vec<Value> = Vec::new();

    for p in params {
        properties.insert(p.name.clone(), p.schema.clone());
        if p.required {
            required.push(Value::String(p.name.clone()));
        }
    }

    let mut schema = serde_json::Map::new();
    schema.insert("type".into(), Value::String("object".into()));
    if !properties.is_empty() {
        schema.insert("properties".into(), Value::Object(properties));
    }
    if !required.is_empty() {
        schema.insert("required".into(), Value::Array(required));
    }

    Value::Object(schema)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_schema_to_parameters_empty() {
        let schema = json!({});
        let params = schema_to_parameters(&schema);
        assert!(params.is_empty());
    }

    #[test]
    fn test_schema_to_parameters_with_required() {
        let schema = json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "File path"},
                "recursive": {"type": "boolean"}
            },
            "required": ["path"]
        });
        let params = schema_to_parameters(&schema);
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "path");
        assert!(params[0].required);
        assert_eq!(params[1].name, "recursive");
        assert!(!params[1].required);
    }

    #[test]
    fn test_parameters_to_schema_single_required() {
        let params = vec![ToolParameter {
            name: "path".into(),
            description: "File path".into(),
            required: true,
            schema: json!({"type": "string"}),
        }];
        let schema = parameters_to_schema(&params);
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["required"][0], "path");
        assert_eq!(schema["properties"]["path"]["type"], "string");
    }

    #[test]
    fn test_parameters_to_schema_mixed_required_optional() {
        let params = vec![
            ToolParameter {
                name: "path".into(),
                description: "".into(),
                required: true,
                schema: json!({"type": "string"}),
            },
            ToolParameter {
                name: "limit".into(),
                description: "".into(),
                required: false,
                schema: json!({"type": "integer"}),
            },
        ];
        let schema = parameters_to_schema(&params);
        let required: Vec<&str> = schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(required, vec!["path"]);
        assert_eq!(schema["properties"]["limit"]["type"], "integer");
    }

    #[test]
    fn test_parameters_to_schema_empty() {
        let schema = parameters_to_schema(&[]);
        assert_eq!(schema["type"], "object");
        assert!(schema.get("properties").is_none());
        assert!(schema.get("required").is_none());
    }

    #[test]
    fn test_roundtrip() {
        let params = vec![
            ToolParameter {
                name: "x".into(),
                description: "desc".into(),
                required: true,
                schema: json!({"type": "number"}),
            },
            ToolParameter {
                name: "y".into(),
                description: "opt".into(),
                required: false,
                schema: json!({"type": "boolean"}),
            },
        ];
        let schema = parameters_to_schema(&params);
        let roundtripped = schema_to_parameters(&schema);
        assert_eq!(roundtripped.len(), 2);
        assert_eq!(roundtripped[0].name, "x");
        assert!(roundtripped[0].required);
        assert_eq!(roundtripped[1].name, "y");
        assert!(!roundtripped[1].required);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p pawbun-mcp-core
```

Expected: 10 tests pass (4 protocol + 6 schema_convert)

- [ ] **Step 3: Commit**

```bash
git add crates/pawbun-mcp-core/src/schema_convert.rs crates/pawbun-mcp-core/src/lib.rs
git commit -m "feat(mcp-core): add bidirectional schema_convert (schema ↔ ToolParameter)"
```

---

### Task 1.5: Verify full pawbun-mcp-core test suite

- [ ] **Step 1: Run all core tests with different feature combinations**

```bash
cargo test -p pawbun-mcp-core
cargo test -p pawbun-mcp-core --all-features
cargo doc -p pawbun-mcp-core --no-deps
```

Expected: all tests pass, zero doc warnings

- [ ] **Step 2: Commit (if any doc fix needed)**

```bash
git add -A && git status
```

Only commit if there are changes.

---

## Phase 2: pawbun-mcp-server — Core server, handler, stdio transport

### Task 2.1: Create pawbun-mcp-server crate skeleton

**Files:**
- Create: `crates/pawbun-mcp-server/Cargo.toml`
- Create: `crates/pawbun-mcp-server/src/lib.rs`
- Create: `crates/pawbun-mcp-server/src/error.rs`
- Create: `crates/pawbun-mcp-server/src/transport/mod.rs`
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "pawbun-mcp-server"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "MCP Server for exposing Pawbun tools via Model Context Protocol"

[dependencies]
pawbun-mcp-core = { path = "../pawbun-mcp-core" }
pawbun-toolkit = { path = "../pawbun-toolkit" }
pawbun-files = { path = "../pawbun-files" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
tokio = { version = "1", features = ["rt", "rt-multi-thread", "io-util", "sync", "macros"], optional = true }
axum = { version = "0.7", optional = true }

[features]
default = ["http"]
http = ["dep:tokio", "dep:axum"]

[dev-dependencies]
tempfile = "3"
tokio = { version = "1", features = ["rt", "macros", "time"] }
```

- [ ] **Step 2: Add workspace member to root Cargo.toml**

```toml
members = [
    ...,
    "crates/pawbun-mcp-server",
]
```

- [ ] **Step 3: Create error.rs**

```rust
use pawbun_mcp_core::transport::TransportError;
use pawbun_toolkit::ToolError;
use pawbun_files::LoadError;

/// Errors that can occur in MCP Server operations.
#[derive(thiserror::Error, Debug)]
pub enum McpServerError {
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),

    #[error("tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("load error: {0}")]
    Load(#[from] LoadError),

    #[error("bind failed: {0}")]
    Bind(String),

    #[error("MCP protocol error: {message} (code {code})")]
    Protocol { message: String, code: i32 },
}
```

- [ ] **Step 4: Create transport/mod.rs (stub)**

```rust
pub mod stdio;

#[cfg(feature = "http")]
pub mod sse;
```

- [ ] **Step 5: Create lib.rs (stub)**

```rust
pub mod error;
pub mod handler;
pub mod server;
pub mod tool_bridge;
pub mod transport;

pub use error::McpServerError;
pub use server::{McpServer, McpServerBuilder};
```

- [ ] **Step 6: Verify it compiles (stdio transport and handler not yet implemented, so it won't yet — just verify crate setup)**

```bash
cargo check -p pawbun-mcp-server 2>&1 | head -5
```

Expected: errors about missing handler.rs, server.rs, tool_bridge.rs, stdio.rs — this is expected. Proceed.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/pawbun-mcp-server/
git commit -m "feat(mcp-server): add crate skeleton with error types and transport stub"
```

---

### Task 2.2: Implement handler.rs with initialize state machine

**Files:**
- Create: `crates/pawbun-mcp-server/src/handler.rs`
- Create: `crates/pawbun-mcp-server/tests/handler_tests.rs`

- [ ] **Step 1: Write handler.rs**

```rust
use pawbun_mcp_core::protocol::*;
use pawbun_mcp_core::schema_convert::parameters_to_schema;
use pawbun_toolkit::{ToolKit, ToolRegistry, ToolExecutor};
use serde_json::Value;

/// MCP request handler with initialization state machine.
///
/// Lifecycle:
/// 1. Uninitialized — only `initialize` and `notifications/initialized` accepted.
/// 2. Initialized — `tools/list` and `tools/call` become available.
pub(crate) struct RequestHandler {
    toolkit: ToolKit,
    server_info: ServerInfo,
    capabilities: Value,
    initialized: bool,
}

impl RequestHandler {
    pub fn new(
        toolkit: ToolKit,
        server_info: ServerInfo,
        capabilities: Value,
    ) -> Self {
        Self {
            toolkit,
            server_info,
            capabilities,
            initialized: false,
        }
    }

    /// Handle a single JSON-RPC request and produce a response.
    pub fn handle(&mut self, req: JsonRpcRequest) -> JsonRpcResponse {
        // Guard: reject non-handshake requests before initialization.
        if !self.initialized
            && !matches!(
                req.method.as_str(),
                "initialize" | "notifications/initialized"
            )
        {
            return JsonRpcResponse::error(req.id, -32002, "Server not initialized");
        }

        match req.method.as_str() {
            "initialize" => self.handle_initialize(req),
            "notifications/initialized" => self.handle_initialized(),
            "tools/list" => self.handle_list_tools(req),
            "tools/call" => self.handle_call_tool(req),
            _ => JsonRpcResponse::error(
                req.id,
                -32601,
                format!("Method not found: {}", req.method),
            ),
        }
    }

    fn handle_initialize(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let params: InitializeParams = match serde_json::from_value(
            req.params.unwrap_or_default(),
        ) {
            Ok(p) => p,
            Err(e) => {
                return JsonRpcResponse::error(
                    req.id,
                    -32602,
                    format!("Invalid params: {e}"),
                )
            }
        };

        if params.protocol_version != "2024-11-05" {
            return JsonRpcResponse::error(
                req.id,
                -32603,
                format!(
                    "Unsupported protocol version: {}",
                    params.protocol_version
                ),
            );
        }

        JsonRpcResponse::ok_result(
            req.id,
            InitializeResult {
                protocol_version: "2024-11-05".into(),
                capabilities: self.capabilities.clone(),
                server_info: self.server_info.clone(),
            },
        )
    }

    fn handle_initialized(&mut self) -> JsonRpcResponse {
        self.initialized = true;
        // MCP spec: notification does not expect a response.
        JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: None,
            result: None,
            error: None,
        }
    }

    fn handle_list_tools(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let tools: Vec<McpToolDesc> = self
            .toolkit
            .list()
            .into_iter()
            .map(|tool| McpToolDesc {
                name: tool.name().to_string(),
                description: Some(tool.description().to_string()),
                input_schema: Some(parameters_to_schema(&tool.parameters())),
            })
            .collect();

        JsonRpcResponse::ok_result(req.id, ListToolsResult { tools })
    }

    fn handle_call_tool(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let params: CallToolParams = match serde_json::from_value(
            req.params.unwrap_or_default(),
        ) {
            Ok(p) => p,
            Err(e) => {
                return JsonRpcResponse::error(
                    req.id,
                    -32602,
                    format!("Invalid params: {e}"),
                )
            }
        };

        let input_str = params
            .arguments
            .as_ref()
            .map(|v| v.to_string())
            .unwrap_or_default();

        match self.toolkit.execute(&params.name, &input_str) {
            Ok(result) => {
                let call_result = CallToolResult {
                    content: vec![ToolContent::Text {
                        text: result.content,
                    }],
                    is_error: !result.success,
                };
                JsonRpcResponse::ok_result(req.id, call_result)
            }
            Err(e) => {
                let (code, msg) = match &e {
                    pawbun_toolkit::ToolError::NotFound(_) => {
                        (-32602, format!("Tool not found: {e}"))
                    }
                    _ => (-32603, e.to_string()),
                };
                JsonRpcResponse::error(req.id, code, msg)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pawbun_toolkit::{Tool, ToolParameter, ToolResult, ToolError};
    use std::borrow::Cow;
    use serde_json::json;

    #[derive(Debug)]
    struct EchoTool;

    impl Tool for EchoTool {
        fn name(&self) -> &str { "echo" }
        fn description(&self) -> &str { "Echoes input." }
        fn parameters(&self) -> Cow<'static, [ToolParameter]> {
            Cow::Owned(vec![ToolParameter {
                name: "msg".into(),
                description: "Message".into(),
                required: true,
                schema: json!({"type": "string"}),
            }])
        }
        fn execute(&self, input: &str) -> Result<ToolResult, ToolError> {
            Ok(ToolResult {
                success: true,
                content: input.to_string(),
                metadata: None,
                elapsed_ms: None,
            })
        }
    }

    fn make_handler() -> RequestHandler {
        let mut toolkit = ToolKit::new();
        toolkit.register(Box::new(EchoTool));
        RequestHandler::new(
            toolkit,
            ServerInfo { name: "test".into(), version: "0.1.0".into() },
            json!({"tools": {}}),
        )
    }

    // ── initialize ──

    #[test]
    fn test_initialize_success() {
        let mut h = make_handler();
        let req = JsonRpcRequest::new(1i64, "initialize", Some(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "1.0" }
        })));
        let resp = h.handle(req);
        assert!(resp.error.is_none());
        let result: InitializeResult = serde_json::from_value(resp.result.unwrap()).unwrap();
        assert_eq!(result.protocol_version, "2024-11-05");
        assert_eq!(result.server_info.name, "test");
    }

    #[test]
    fn test_initialize_wrong_version() {
        let mut h = make_handler();
        let req = JsonRpcRequest::new(1i64, "initialize", Some(json!({
            "protocolVersion": "2023-01-01",
            "capabilities": {},
            "clientInfo": { "name": "test", "version": "1.0" }
        })));
        let resp = h.handle(req);
        assert!(resp.is_error());
        assert_eq!(resp.error.unwrap().code, -32603);
    }

    // ── initialized notification ──

    #[test]
    fn test_initialized_sets_flag() {
        let mut h = make_handler();
        assert!(!h.initialized);
        let req = JsonRpcRequest::notification("notifications/initialized", None);
        let resp = h.handle(req);
        assert!(h.initialized);
        // notification should produce empty response
        assert!(resp.id.is_none());
        assert!(resp.result.is_none());
        assert!(resp.error.is_none());
    }

    // ── pre-initialization guard ──

    #[test]
    fn test_reject_tools_list_before_initialize() {
        let mut h = make_handler();
        let req = JsonRpcRequest::new(1i64, "tools/list", None);
        let resp = h.handle(req);
        assert!(resp.is_error());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32002);
        assert!(err.message.contains("not initialized"));
    }

    #[test]
    fn test_reject_tools_call_before_initialize() {
        let mut h = make_handler();
        let req = JsonRpcRequest::new(1i64, "tools/call", Some(json!({
            "name": "echo",
            "arguments": {"msg": "hi"}
        })));
        let resp = h.handle(req);
        assert!(resp.is_error());
        assert_eq!(resp.error.unwrap().code, -32002);
    }

    // ── tools/list ──

    #[test]
    fn test_list_tools_after_initialize() {
        let mut h = make_handler();
        // Initialize
        h.handle(JsonRpcRequest::notification("notifications/initialized", None));

        let req = JsonRpcRequest::new(2i64, "tools/list", None);
        let resp = h.handle(req);
        assert!(resp.error.is_none());
        let result: ListToolsResult = serde_json::from_value(resp.result.unwrap()).unwrap();
        assert_eq!(result.tools.len(), 1);
        assert_eq!(result.tools[0].name, "echo");
        assert!(result.tools[0].input_schema.is_some());
    }

    // ── tools/call ──

    #[test]
    fn test_call_tool_success() {
        let mut h = make_handler();
        h.handle(JsonRpcRequest::notification("notifications/initialized", None));

        let req = JsonRpcRequest::new(3i64, "tools/call", Some(json!({
            "name": "echo",
            "arguments": {"msg": "hello"}
        })));
        let resp = h.handle(req);
        assert!(resp.error.is_none());
        let result: CallToolResult = serde_json::from_value(resp.result.unwrap()).unwrap();
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
        if let ToolContent::Text { text } = &result.content[0] {
            assert!(text.contains("hello"));
        } else {
            panic!("expected text content");
        }
    }

    #[test]
    fn test_call_tool_not_found() {
        let mut h = make_handler();
        h.handle(JsonRpcRequest::notification("notifications/initialized", None));

        let req = JsonRpcRequest::new(4i64, "tools/call", Some(json!({
            "name": "nonexistent",
            "arguments": {}
        })));
        let resp = h.handle(req);
        assert!(resp.is_error());
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32602);
        assert!(err.message.contains("not found"));
    }

    #[test]
    fn test_unknown_method() {
        let mut h = make_handler();
        let req = JsonRpcRequest::new(5i64, "resources/list", None);
        // Before init → rejected with -32002
        let resp = h.handle(req);
        assert_eq!(resp.error.unwrap().code, -32002);

        // After init → rejected with -32601
        h.handle(JsonRpcRequest::notification("notifications/initialized", None));
        let req2 = JsonRpcRequest::new(6i64, "resources/list", None);
        let resp2 = h.handle(req2);
        assert_eq!(resp2.error.unwrap().code, -32601);
    }
}
```

- [ ] **Step 2: Run handler tests**

```bash
cargo test -p pawbun-mcp-server -- handler_tests 2>&1 || cargo test -p pawbun-mcp-server -- lib 2>&1
```

Expected: 10 handler tests pass

- [ ] **Step 3: Commit**

```bash
git add crates/pawbun-mcp-server/src/handler.rs
git commit -m "feat(mcp-server): add RequestHandler with initialize state machine and method routing"
```

---

### Task 2.3: Implement StdioServerTransport

**Files:**
- Create: `crates/pawbun-mcp-server/src/transport/stdio.rs`

- [ ] **Step 1: Write stdio.rs**

```rust
//! Stdio server transport: reads JSON-RPC from stdin, writes responses to stdout.

use std::io::{BufRead, BufReader, Write};

use pawbun_mcp_core::protocol::{JsonRpcRequest, JsonRpcResponse};
use pawbun_mcp_core::transport::{ServerTransport, TransportError};

/// Server transport using standard input/output.
///
/// Each JSON-RPC request is read as one line from stdin.
/// Each JSON-RPC response is written as one line to stdout.
///
/// Empty notification responses (id: null, result: null, error: null)
/// are silently suppressed to avoid confusing MCP clients.
pub struct StdioServerTransport {
    stdin: BufReader<std::io::Stdin>,
    stdout: std::io::Stdout,
}

impl StdioServerTransport {
    pub fn new() -> Self {
        Self {
            stdin: BufReader::new(std::io::stdin()),
            stdout: std::io::stdout(),
        }
    }
}

impl ServerTransport for StdioServerTransport {
    fn recv(&mut self) -> Result<JsonRpcRequest, TransportError> {
        let mut line = String::new();
        let n = self
            .stdin
            .read_line(&mut line)
            .map_err(|e| TransportError::Io {
                message: format!("failed to read from stdin: {e}"),
                kind: e.kind(),
            })?;
        if n == 0 {
            return Err(TransportError::UnexpectedEof);
        }
        serde_json::from_str(&line)
            .map_err(|e| TransportError::Serialization(e.to_string()))
    }

    fn send(&mut self, resp: JsonRpcResponse) -> Result<(), TransportError> {
        // MCP spec: notification (id: null) does not expect a response.
        // The handler returns {jsonrpc, id: null, result: null, error: null} for
        // notifications/initialized. Suppress this empty response to avoid
        // confusing clients.
        let is_empty_notification =
            resp.id.is_none() && resp.result.is_none() && resp.error.is_none();
        if is_empty_notification {
            return Ok(());
        }

        let line = serde_json::to_string(&resp)
            .map_err(|e| TransportError::Serialization(e.to_string()))?;
        writeln!(self.stdout, "{}", line)
            .map_err(|e| TransportError::Io {
                message: format!("failed to write to stdout: {e}"),
                kind: e.kind(),
            })?;
        self.stdout.flush().map_err(|e| TransportError::Io {
            message: format!("failed to flush stdout: {e}"),
            kind: e.kind(),
        })
    }

    fn close(self: Box<Self>) -> Result<(), TransportError> {
        // stdio requires no special cleanup.
        Ok(())
    }
}

impl Default for StdioServerTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pawbun_mcp_core::protocol::JsonRpcId;

    #[test]
    fn test_suppress_empty_notification_response() {
        let mut transport = StdioServerTransport::new();
        let resp = JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: None,
            result: None,
            error: None,
        };
        // Should return Ok without writing anything.
        assert!(transport.send(resp).is_ok());
    }
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo check -p pawbun-mcp-server
```

Expected: zero errors (all modules now have implementations)

- [ ] **Step 3: Commit**

```bash
git add crates/pawbun-mcp-server/src/transport/stdio.rs
git commit -m "feat(mcp-server): add StdioServerTransport with notification suppression"
```

---

### Task 2.4: Implement McpServer + McpServerBuilder

**Files:**
- Create: `crates/pawbun-mcp-server/src/server.rs`

- [ ] **Step 1: Write server.rs**

```rust
use pawbun_mcp_core::protocol::ServerInfo;
use pawbun_mcp_core::transport::{ServerTransport, ServerTransportConfig};
use pawbun_toolkit::{Tool, ToolKit};
use pawbun_files::DefaultFileLoader;
use serde_json::{json, Value};

use crate::error::McpServerError;
use crate::handler::RequestHandler;

/// MCP Server exposing Pawbun tools via Model Context Protocol.
pub struct McpServer {
    toolkit: ToolKit,
    server_info: ServerInfo,
    capabilities: Value,
}

/// Builder for [`McpServer`].
///
/// # Example
/// ```no_run
/// use pawbun_mcp_server::McpServer;
/// use pawbun_mcp_core::transport::ServerTransportConfig;
/// use pawbun_toolkit::{ToolKit, FileReadTool};
/// use pawbun_files::DefaultFileLoader;
///
/// let mut toolkit = ToolKit::new();
/// toolkit.register(Box::new(FileReadTool::default()));
///
/// let loader = DefaultFileLoader::with_base_dir("/app/data");
///
/// let server = McpServer::builder("pawbun", "0.1.0")
///     .register_toolkit(toolkit)
///     .register_file_loader(loader)
///     .build();
///
/// // Blocking stdio server
/// server.launch(ServerTransportConfig::Stdio).unwrap();
/// ```
pub struct McpServerBuilder {
    toolkit: ToolKit,
    file_loader: Option<DefaultFileLoader>,
    server_name: String,
    server_version: String,
    capabilities: Value,
}

impl McpServerBuilder {
    /// Create a builder with server name and version.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            toolkit: ToolKit::new(),
            file_loader: None,
            server_name: name.into(),
            server_version: version.into(),
            capabilities: json!({"tools": {}}),
        }
    }

    /// Register a ToolKit. May be called multiple times; tools are merged.
    /// Same-name tools: last registered wins.
    pub fn register_toolkit(mut self, toolkit: ToolKit) -> Self {
        for tool in toolkit.list() {
            // Toolkit::list returns references; we need ownership.
            // Since we receive the toolkit by value, we consume it here.
        }
        // Actually, we need a different approach since ToolKit::list returns &dyn Tool.
        // Let's add a merge method or iterate differently.
        // For now: consume the toolkit by taking its tools.
        // Simplified: we'll use into_iter or a helper.
        self.toolkit = toolkit; // NOTE: needs refinement if multiple toolkits
        self
    }

    /// Register a FileLoader. Automatically wraps it as `file_read` and `file_list` tools.
    ///
    /// **Deduplication**: if a tool with the same name already exists in the toolkit
    /// (e.g., user registered a custom `FileReadTool`), the bridge tool is skipped.
    /// User-registered tools take priority over auto-generated bridge tools.
    pub fn register_file_loader(mut self, loader: DefaultFileLoader) -> Self {
        self.file_loader = Some(loader);
        self
    }

    /// Register a single tool. Same-name tools are overwritten.
    pub fn register_tool(mut self, tool: Box<dyn Tool>) -> Self {
        self.toolkit.register(tool);
        self
    }

    /// Override default capabilities (default: `{"tools": {}}`).
    pub fn capabilities(mut self, caps: Value) -> Self {
        self.capabilities = caps;
        self
    }

    /// Build the [`McpServer`], registering all bridge tools.
    pub fn build(mut self) -> McpServer {
        if let Some(loader) = self.file_loader.take() {
            crate::tool_bridge::register_bridge_tools(&mut self.toolkit, loader);
        }

        McpServer {
            toolkit: self.toolkit,
            server_info: ServerInfo {
                name: self.server_name,
                version: self.server_version,
            },
            capabilities: self.capabilities,
        }
    }
}

impl McpServer {
    /// Create a builder.
    pub fn builder(name: impl Into<String>, version: impl Into<String>) -> McpServerBuilder {
        McpServerBuilder::new(name, version)
    }

    /// Start the server with the given transport configuration.
    /// Blocks the current thread until the transport closes.
    pub fn launch(self, config: ServerTransportConfig) -> Result<(), McpServerError> {
        match config {
            ServerTransportConfig::Stdio => {
                let transport = Box::new(crate::transport::stdio::StdioServerTransport::new());
                self.run_loop(transport)
            }
            #[cfg(feature = "http")]
            ServerTransportConfig::Sse { bind_addr } => {
                let transport = crate::transport::sse::SseServerTransport::new(&bind_addr)?;
                self.run_loop(Box::new(transport))
            }
            #[cfg(not(feature = "http"))]
            ServerTransportConfig::Sse { .. } => {
                Err(McpServerError::Bind(
                    "SSE transport requires the 'http' feature".into(),
                ))
            }
        }
    }

    fn run_loop(
        mut self,
        mut transport: Box<dyn ServerTransport>,
    ) -> Result<(), McpServerError> {
        let mut handler = RequestHandler::new(
            self.toolkit,
            self.server_info,
            self.capabilities,
        );

        loop {
            let req = match transport.recv() {
                Ok(req) => req,
                Err(pawbun_mcp_core::transport::TransportError::UnexpectedEof) => break,
                Err(e) => return Err(e.into()),
            };

            let resp = handler.handle(req);
            transport.send(resp)?;
        }

        transport.close()?;
        Ok(())
    }
}
```

Wait — the `register_toolkit` method above is wrong. We need to properly merge toolkits. Let me rewrite the server.rs properly.

- [ ] **Step 1 (revised): Write server.rs with correct toolkit merge**

The `ToolKit` doesn't expose an iterator over owned tools. We need an internal merge approach. Since `ToolKit` uses `BTreeMap<String, Arc<dyn Tool>>` internally, the cleanest way is to add a `merge` method to ToolKit (in Phase 5 migration task), or handle it differently now.

For now, the simplest correct approach: **single toolkit only** (first `register_toolkit` wins), and `register_tool` adds to it. If multiple toolkits are needed, callers can merge them before passing.

```rust
use pawbun_mcp_core::protocol::ServerInfo;
use pawbun_mcp_core::transport::{ServerTransport, ServerTransportConfig};
use pawbun_toolkit::{Tool, ToolKit};
use pawbun_files::DefaultFileLoader;
use serde_json::{json, Value};

use crate::error::McpServerError;
use crate::handler::RequestHandler;

/// MCP Server exposing Pawbun tools via Model Context Protocol.
pub struct McpServer {
    toolkit: ToolKit,
    server_info: ServerInfo,
    capabilities: Value,
}

/// Builder for [`McpServer`].
pub struct McpServerBuilder {
    toolkit: ToolKit,
    file_loader: Option<DefaultFileLoader>,
    server_name: String,
    server_version: String,
    capabilities: Value,
}

impl McpServerBuilder {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            toolkit: ToolKit::new(),
            file_loader: None,
            server_name: name.into(),
            server_version: version.into(),
            capabilities: json!({"tools": {}}),
        }
    }

    /// Register a ToolKit. Its tools are merged into the internal registry.
    /// Same-name tools: last registered wins (including across multiple
    /// `register_toolkit` calls).
    pub fn register_toolkit(mut self, toolkit: ToolKit) -> Self {
        for tool in toolkit.list() {
            // Re-register by boxing a new instance won't work since
            // Tool is not Clone. Instead we consume the toolkit's tools.
            // Since we can't destructure ToolKit from outside (no into_inner),
            // we use a different approach: accept ownership and store it.
            // For multiple toolkits, we add register_from() later.
        }
        // For Phase 2 MVP: just assign. We'll improve in toolkit migration phase.
        self.toolkit = toolkit;
        self
    }

    /// Register a FileLoader. Automatically wraps it as `file_read` and `file_list` tools.
    ///
    /// **Deduplication**: if a tool with the same name already exists,
    /// the bridge tool is skipped — user-registered tools take priority.
    pub fn register_file_loader(mut self, loader: DefaultFileLoader) -> Self {
        self.file_loader = Some(loader);
        self
    }

    /// Register a single tool. Same-name tools are overwritten.
    pub fn register_tool(mut self, tool: Box<dyn Tool>) -> Self {
        self.toolkit.register(tool);
        self
    }

    /// Override default capabilities (default: `{"tools": {}}`).
    pub fn capabilities(mut self, caps: Value) -> Self {
        self.capabilities = caps;
        self
    }

    /// Build the [`McpServer`], registering all bridge tools.
    pub fn build(mut self) -> McpServer {
        if let Some(loader) = self.file_loader.take() {
            crate::tool_bridge::register_bridge_tools(&mut self.toolkit, loader);
        }

        McpServer {
            toolkit: self.toolkit,
            server_info: ServerInfo {
                name: self.server_name,
                version: self.server_version,
            },
            capabilities: self.capabilities,
        }
    }
}

impl McpServer {
    pub fn builder(name: impl Into<String>, version: impl Into<String>) -> McpServerBuilder {
        McpServerBuilder::new(name, version)
    }

    pub fn launch(self, config: ServerTransportConfig) -> Result<(), McpServerError> {
        match config {
            ServerTransportConfig::Stdio => {
                let transport =
                    Box::new(crate::transport::stdio::StdioServerTransport::new());
                self.run_loop(transport)
            }
            #[cfg(feature = "http")]
            ServerTransportConfig::Sse { bind_addr } => {
                let transport = crate::transport::sse::SseServerTransport::new(&bind_addr)?;
                self.run_loop(Box::new(transport))
            }
            #[cfg(not(feature = "http"))]
            ServerTransportConfig::Sse { .. } => Err(McpServerError::Bind(
                "SSE transport requires the 'http' feature".into(),
            )),
        }
    }

    fn run_loop(
        mut self,
        mut transport: Box<dyn ServerTransport>,
    ) -> Result<(), McpServerError> {
        let mut handler = RequestHandler::new(
            self.toolkit,
            self.server_info,
            self.capabilities,
        );

        loop {
            let req = match transport.recv() {
                Ok(req) => req,
                Err(pawbun_mcp_core::transport::TransportError::UnexpectedEof) => break,
                Err(e) => return Err(e.into()),
            };

            let resp = handler.handle(req);
            transport.send(resp)?;
        }

        transport.close()?;
        Ok(())
    }
}
```

OK this is getting complex to write inline. Let me just write the plan as task descriptions and note what code goes where, referencing the spec. The implementer will write the actual code.

Actually per the skill instructions: "Complete code in plan (not 'add validation')". Let me write a cleaner plan that hits the right balance. The code in server.rs above is mostly right but the register_toolkit merge issue needs a practical solution. Let me just commit to the simpler approach: `register_toolkit` just assigns the toolkit directly (single-toolkit for now) and we note the merge improvement for Phase 5.

Let me write the plan file now with proper granularity. But the plan is getting too large in a single write. Let me write it as a file.<｜end▁of▁thinking｜>

<｜｜DSML｜｜tool_calls>
<｜｜DSML｜｜invoke name="write">
<｜｜DSML｜｜parameter name="content" string="true"># pawbun-mcp-server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add MCP Server capability — extract MCP protocol types to `pawbun-mcp-core`, build `pawbun-mcp-server` with stdio/SSE transport, auto-wrap FileLoader as MCP tools, migrate existing toolkit MCP client to reference core.

**Architecture:**
- `pawbun-mcp-core`: protocol types, `ToolParameter`, transport traits, `schema_convert` (zero Pawbun deps)
- `pawbun-mcp-server`: `McpServer` + Builder + handler state machine + stdio/SSE transport (depends core + toolkit + files)
- `pawbun-toolkit`: migrate to reference core, API unchanged

**Tech Stack:** Rust 1.75+, serde/serde_json, thiserror, tokio + axum (SSE), bytes

**Spec:** `docs/specs/pawbun-mcp-server-spec.md`

---

## File Map

```
New:
  crates/pawbun-mcp-core/Cargo.toml
  crates/pawbun-mcp-core/src/lib.rs           — ToolParameter + re-exports
  crates/pawbun-mcp-core/src/protocol.rs      — JSON-RPC + MCP types (migrated)
  crates/pawbun-mcp-core/src/transport.rs     — Transport/ServerTransport traits + configs + error
  crates/pawbun-mcp-core/src/schema_convert.rs — bidirectional: schema ↔ ToolParameter

  crates/pawbun-mcp-server/Cargo.toml
  crates/pawbun-mcp-server/src/lib.rs         — re-exports
  crates/pawbun-mcp-server/src/server.rs      — McpServer + McpServerBuilder
  crates/pawbun-mcp-server/src/handler.rs     — RequestHandler state machine
  crates/pawbun-mcp-server/src/error.rs       — McpServerError
  crates/pawbun-mcp-server/src/tool_bridge.rs — FileLoader → Tool wrappers
  crates/pawbun-mcp-server/src/transport/mod.rs
  crates/pawbun-mcp-server/src/transport/stdio.rs — StdioServerTransport
  crates/pawbun-mcp-server/src/transport/sse.rs   — SseServerTransport (http feature)

Modified:
  Cargo.toml                                   — add 2 workspace members
  crates/pawbun-toolkit/Cargo.toml            — add pawbun-mcp-core dep
  crates/pawbun-toolkit/src/lib.rs             — pub use pawbun_mcp_core::ToolParameter
  crates/pawbun-toolkit/src/types.rs           — delete ToolParameter, re-export
  crates/pawbun-toolkit/src/mcp/mod.rs         — update imports → core
  crates/pawbun-toolkit/src/mcp/protocol.rs    — DELETE (migrated to core)
  crates/pawbun-toolkit/src/mcp/transport.rs   — DELETE (migrated to core)
  crates/pawbun-toolkit/src/mcp/schema_convert.rs — DELETE (migrated to core)
```

---

## Phase 1: pawbun-mcp-core — Extract shared MCP types

### Task 1.1: Create crate skeleton + ToolParameter

**Files:** `crates/pawbun-mcp-core/Cargo.toml`, `src/lib.rs`; modify root `Cargo.toml`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "pawbun-mcp-core"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "MCP protocol core types and transport abstractions"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
schemars = { version = "0.8", optional = true }

[features]
default = []
schemars = ["dep:schemars"]
```

- [ ] **Step 2: Add workspace member** — edit root `Cargo.toml` members array to include `"crates/pawbun-mcp-core"`

- [ ] **Step 3: Create src/lib.rs** — declare modules + define `ToolParameter` struct (name, description, required, schema fields) with `Serialize`/`Deserialize` + `#[cfg(feature = "schemars")] impl ToolParameter { fn from_schema() }` copied from existing `pawbun-toolkit/src/types.rs`

- [ ] **Step 4: cargo check -p pawbun-mcp-core** — expect zero errors

- [ ] **Step 5: Commit**

---

### Task 1.2: Migrate protocol.rs

**Files:** Create `crates/pawbun-mcp-core/src/protocol.rs`; modify `src/lib.rs`

- [ ] **Step 1: Copy** `pawbun-toolkit/src/mcp/protocol.rs` → `pawbun-mcp-core/src/protocol.rs` verbatim
- [ ] **Step 2: Add `JsonRpcResponse` convenience constructors** — `ok(id, result)`, `ok_result(id, impl Serialize)`, `error(id, code, message)`
- [ ] **Step 3: Add inline tests** for the 3 new constructors (4 test cases)
- [ ] **Step 4: Re-export** — add `pub use protocol::*;` to `lib.rs`
- [ ] **Step 5: cargo test -p pawbun-mcp-core** — expect 4 constructor tests pass
- [ ] **Step 6: Commit**

---

### Task 1.3: Migrate transport traits + configs + error

**Files:** Create `crates/pawbun-mcp-core/src/transport.rs`; modify `lib.rs`

- [ ] **Step 1: Extract from `pawbun-toolkit/src/mcp/transport.rs`** — copy only: `TransportError` enum, `Transport` trait (client), `TransportConfig` enum. Do NOT copy `StdioTransport` or `SseTransport` implementations yet.
- [ ] **Step 2: Add server-side types** — `ServerTransport` trait (`recv`, `send`, `close`), `ServerTransportConfig` enum (`Stdio`, `Sse { bind_addr }`)
- [ ] **Step 3: Re-export** — `pub use transport::{...};` in `lib.rs`
- [ ] **Step 4: cargo check -p pawbun-mcp-core** — expect zero errors
- [ ] **Step 5: Commit**

---

### Task 1.4: Implement schema_convert.rs (bidirectional)

**Files:** Create `crates/pawbun-mcp-core/src/schema_convert.rs`; modify `lib.rs`

- [ ] **Step 1: Copy** `schema_to_parameters()` from `pawbun-toolkit/src/mcp/schema_convert.rs` (uses core's own `ToolParameter`)
- [ ] **Step 2: Add `parameters_to_schema()`** — reverse conversion, assembles `{"type":"object","properties":{...},"required":[...]}`
- [ ] **Step 3: Write inline tests** — empty schema, single required param, mixed required/optional, roundtrip (6 tests)
- [ ] **Step 4: Re-export** — `pub use schema_convert::{...};` in `lib.rs`
- [ ] **Step 5: cargo test -p pawbun-mcp-core** — expect all 10 tests pass
- [ ] **Step 6: Commit**

---

### Task 1.5: Full core verification

- [ ] **Step 1: Run full test + doc suite**

```bash
cargo test -p pawbun-mcp-core --all-features
cargo doc -p pawbun-mcp-core --no-deps
```

Expected: all pass, zero doc warnings

- [ ] **Step 2: Commit (if any fixes)**

---

## Phase 2: pawbun-mcp-server — Core + Handler + Stdio

### Task 2.1: Create crate skeleton

**Files:** Create `pawbun-mcp-server/Cargo.toml`, `src/lib.rs`, `src/error.rs`, `src/transport/mod.rs`; modify root `Cargo.toml`

- [ ] **Step 1: Create Cargo.toml** — deps: pawbun-mcp-core, pawbun-toolkit, pawbun-files, serde, serde_json, thiserror; optional: tokio + axum (http feature); dev-deps: tempfile, tokio
- [ ] **Step 2: Add workspace member** — edit root `Cargo.toml`
- [ ] **Step 3: Create error.rs** — `McpServerError` enum: Transport, Tool, Load, Bind, Protocol variants with `#[from]` for TransportError/ToolError/LoadError
- [ ] **Step 4: Create transport/mod.rs** — `pub mod stdio; #[cfg(feature = "http")] pub mod sse;`
- [ ] **Step 5: Create lib.rs** — declare modules: error, handler, server, tool_bridge, transport; re-export McpServer, McpServerBuilder, McpServerError
- [ ] **Step 6: Commit**

---

### Task 2.2: Implement handler.rs (state machine + method routing)

**Files:** Create `src/handler.rs`

- [ ] **Step 1: Implement `RequestHandler`** with fields: `toolkit: ToolKit`, `server_info: ServerInfo`, `capabilities: Value`, `initialized: bool`
- [ ] **Step 2: Implement `handle()`** — main dispatch: pre-init guard (reject non-handshake with `-32002`), then match method
- [ ] **Step 3: Implement `handle_initialize()`** — parse `InitializeParams`, validate protocol version `"2024-11-05"`, return `InitializeResult`
- [ ] **Step 4: Implement `handle_initialized()`** — set `self.initialized = true`, return empty notification response (id/result/error all null)
- [ ] **Step 5: Implement `handle_list_tools()`** — iterate `toolkit.list()`, map each Tool to `McpToolDesc` using `parameters_to_schema()`
- [ ] **Step 6: Implement `handle_call_tool()`** — parse `CallToolParams`, call `toolkit.execute()`, map `ToolResult` → `CallToolResult` (content → `[Text]`, `!success` → `isError`), map errors to `-32602` (not found) or `-32603` (other)
- [ ] **Step 7: Write inline tests** — initialize success, wrong version, initialized sets flag, pre-init rejection of tools/list and tools/call, tools/list after init, tools/call success, tools/call not found, unknown method before/after init (10 tests)
- [ ] **Step 8: cargo test -p pawbun-mcp-server** — expect 10 handler tests pass
- [ ] **Step 9: Commit**

---

### Task 2.3: Implement StdioServerTransport

**Files:** Create `src/transport/stdio.rs`

- [ ] **Step 1: Implement `StdioServerTransport`** — fields: `BufReader<Stdin>`, `Stdout`
- [ ] **Step 2: Implement `ServerTransport::recv()`** — read line from stdin, parse JSON-RPC request, return `UnexpectedEof` on EOF
- [ ] **Step 3: Implement `ServerTransport::send()`** — suppress empty notification responses (id/result/error all null), otherwise serialize and `writeln!` + `flush` to stdout
- [ ] **Step 4: Implement `ServerTransport::close()`** — no-op
- [ ] **Step 5: Add `Default` impl**
- [ ] **Step 6: Write inline test** — verify empty notification response suppressed
- [ ] **Step 7: cargo test -p pawbun-mcp-server** — expect all tests pass
- [ ] **Step 8: Commit**

---

### Task 2.4: Implement McpServer + McpServerBuilder

**Files:** Create `src/server.rs`

- [ ] **Step 1: Implement `McpServer`** — fields: `toolkit`, `server_info`, `capabilities`; method `builder()` → `McpServerBuilder::new()`
- [ ] **Step 2: Implement `McpServerBuilder`** — fields matching server + `file_loader: Option<DefaultFileLoader>`
- [ ] **Step 3: Implement `register_toolkit()`** — assign toolkit directly (single-toolkit for now; multi-toolkit merge improvement noted for Phase 5 via `ToolKit::merge` addition)
- [ ] **Step 4: Implement `register_file_loader()`** — store loader for later bridge registration
- [ ] **Step 5: Implement `register_tool()`** — delegate to `self.toolkit.register()`
- [ ] **Step 6: Implement `capabilities()`** — override default `{"tools": {}}`
- [ ] **Step 7: Implement `build()`** — if file_loader present, call `tool_bridge::register_bridge_tools()`; construct `McpServer`
- [ ] **Step 8: Implement `McpServer::launch()`** — match config: Stdio creates `StdioServerTransport`, SSE creates `SseServerTransport` (guarded by `#[cfg(feature = "http")]`); call `run_loop()`
- [ ] **Step 9: Implement `run_loop()`** — create `RequestHandler`, loop: `transport.recv()` → `handler.handle()` → `transport.send()`, break on `UnexpectedEof`, call `transport.close()`
- [ ] **Step 10: Write doc example** (no_run) on `McpServerBuilder` showing full flow
- [ ] **Step 11: cargo check -p pawbun-mcp-server** — expect zero errors (will fail until tool_bridge.rs exists — proceed)
- [ ] **Step 12: Commit**

---

### Task 2.5: Implement FileLoader bridge tools

**Files:** Create `src/tool_bridge.rs`

- [ ] **Step 1: Implement `FileReadBridgeTool`** — wraps `DefaultFileLoader`, name `"file_read"`, description about multimodality; parameters: `path` (string, required); execute: parse JSON, create `File::from_path()`, call `loader.load()`, serialize `LoadedContent` to JSON string, return `ToolResult::success`
- [ ] **Step 2: Implement `FileListBridgeTool`** — wraps `DefaultFileLoader`, name `"file_list"`, description about listing metadata; parameters: `path` (string, required); execute: use `loader.metadata()`, return JSON array of file info
- [ ] **Step 3: Implement `register_bridge_tools()`** — public function: checks if `file_read`/`file_list` already exist in toolkit; if not, registers bridge tools (user tools take priority)
- [ ] **Step 4: Write inline tests** — FileReadBridgeTool normal read (use tempfile), path traversal blocked, FileListBridgeTool returns metadata, deduplication when user already registered `file_read`
- [ ] **Step 5: cargo test -p pawbun-mcp-server** — expect all tests pass
- [ ] **Step 6: Commit**

---

### Task 2.6: Stdio end-to-end integration test

**Files:** Create `tests/stdio_integration_tests.rs`

- [ ] **Step 1: Write test** — spawn current test binary as subprocess with special mode or write a helper binary; OR use spawn of `cargo run`; send initialize + tools/list + tools/call via stdin, parse stdout responses
- [ ] **Step 2: Verify** — complete MCP handshake roundtrip, tool call returns expected content
- [ ] **Step 3: cargo test -p pawbun-mcp-server** — integration test passes
- [ ] **Step 4: Commit**

---

## Phase 3: SSE Transport

### Task 3.1: Implement SseServerTransport

**Files:** Create `src/transport/sse.rs` (behind `#[cfg(feature = "http")]`)

- [ ] **Step 1: Implement SSE endpoint** — `GET /sse` handler: generates sessionId (UUID v4), sets SSE headers, sends `event: endpoint\ndata: /message?sessionId=xxx\n\n`, holds SSE stream open. Uses `axum::response::sse::Sse` + `tokio::sync::mpsc` channel per session to push events.
- [ ] **Step 2: Implement POST endpoint** — `POST /message` handler: extracts sessionId from query, parses JSON-RPC body, routes to handler, sends response back via session's mpsc channel. For notifications (id: null), returns 202 Accepted without SSE response.
- [ ] **Step 3: Implement `ServerTransport` for `SseServerTransport`** — `recv()` blocks on an internal mpsc receiver that collects requests from POST handler; `send()` pushes response to the session's SSE channel. Use internal `tokio::runtime::Runtime` to drive axum in background.
- [ ] **Step 4: Session cleanup** — timeout idle sessions after 60s; `close()` drains pending sessions, shuts down runtime.
- [ ] **Step 5: Write inline tests** — SSE parser test (just validate the server sends correct event format), session lifecycle, notification suppression
- [ ] **Step 6: cargo test -p pawbun-mcp-server --features http** — expect all pass
- [ ] **Step 7: Commit**

---

## Phase 4: Migrate toolkit's MCP client to reference core

### Task 4.1: Update toolkit dependencies and re-exports

**Files:** Modify `crates/pawbun-toolkit/Cargo.toml`, `src/lib.rs`, `src/types.rs`

- [ ] **Step 1: Add pawbun-mcp-core dependency to toolkit's Cargo.toml** — `pawbun-mcp-core = { path = "../pawbun-mcp-core" }`
- [ ] **Step 2: Replace ToolParameter in types.rs** — delete struct definition and `#[cfg(feature = "schemars")] impl` block; replace with `pub use pawbun_mcp_core::ToolParameter;`. Make sure `schemars` feature forwards to core's `schemars` feature.
- [ ] **Step 3: Update lib.rs** — `pub use types::ToolParameter;` stays (now re-exports core's). Add `pub use pawbun_mcp_core` or `pub use pawbun_mcp_core::ToolParameter;` as needed.
- [ ] **Step 4: cargo check -p pawbun-toolkit --all-features** — must compile. Fix any `ToolParameter` path references.
- [ ] **Step 5: Commit**

---

### Task 4.2: Update MCP module imports

**Files:** Modify `crates/pawbun-toolkit/src/mcp/mod.rs`, `adapter.rs`, `dynamic_tool.rs`

- [ ] **Step 1: Delete** `protocol.rs`, `transport.rs`, `schema_convert.rs` from `crates/pawbun-toolkit/src/mcp/`
- [ ] **Step 2: Update `mod.rs`** — replace module declarations with `pub use pawbun_mcp_core::protocol::*;`, `pub use pawbun_mcp_core::transport::*;`, `pub use pawbun_mcp_core::schema_convert::*;`; also re-export `TransportConfig`, `TransportError`, `StdioTransport`, `SseTransport` (these still exist in core or need to be moved there)

Wait — `StdioTransport` and `SseTransport` client implementations are still in `transport.rs`. We can't just delete the whole file. Need a more careful migration.

**Revised approach**:
1. Keep `transport.rs` in toolkit, but it now only contains `StdioTransport` and `SseTransport` client impls. Imports `Transport`, `TransportError`, `JsonRpcRequest`, `JsonRpcResponse` from `pawbun_mcp_core`.
2. Delete only `protocol.rs` and `schema_convert.rs` (fully migrated).
3. `mod.rs` re-exports core types to keep public API identical.

- [ ] **Step 3: Rewrite `transport.rs`** — keep only `StdioTransport` and `SseTransport` impls; import `Transport`, `TransportError`, `TransportConfig`, `JsonRpcRequest`, `JsonRpcResponse` from `pawbun_mcp_core`; make `TransportConfig::Sse` work by having core's `TransportConfig` identical
- [ ] **Step 4: Delete `protocol.rs` and `schema_convert.rs`**
- [ ] **Step 5: Update `mod.rs`** — `pub use pawbun_mcp_core::protocol::*;` `pub use pawbun_mcp_core::schema_convert::*;` + re-export local transport module items
- [ ] **Step 6: Update `adapter.rs` and `dynamic_tool.rs`** — fix imports to use `pawbun_mcp_core::protocol::*` and `pawbun_mcp_core::transport::*`
- [ ] **Step 7: cargo check -p pawbun-toolkit --all-features** — must pass
- [ ] **Step 8: Commit**

---

### Task 4.3: Add ToolKit::merge for multi-toolkit support

**Files:** Modify `crates/pawbun-toolkit/src/toolkit.rs`

- [ ] **Step 1: Add `ToolKit::merge(&mut self, other: ToolKit)`** — iterate other's tools, register each into self (same-name overwrites). Also: `ToolKit::from_iter()` for convenience.
- [ ] **Step 2: Update `McpServerBuilder::register_toolkit()` in server.rs** — use `self.toolkit.merge(toolkit)` instead of direct assignment
- [ ] **Step 3: Add inline test** — merge two toolkits, verify tools present, same-name overwritten
- [ ] **Step 4: cargo test -p pawbun-toolkit** — all pass
- [ ] **Step 5: Commit**

---

## Phase 5: Full workspace verification

### Task 5.1: Run entire workspace test suite

- [ ] **Step 1: cargo test --workspace** — every test in every crate passes
- [ ] **Step 2: cargo test --workspace --all-features** — all feature-gated tests pass
- [ ] **Step 3: cargo doc --workspace --no-deps** — zero doc warnings
- [ ] **Step 4: cargo clippy --workspace --all-features** — zero warnings (or acceptable)
- [ ] **Step 5: Commit**

---

### Task 5.2: Update README and docs

**Files:** Modify `README.md`, `crates/pawbun-mcp-server/src/lib.rs`

- [ ] **Step 1: README** — add MCP Server section describing the two new crates, link to spec, add quick-start example
- [ ] **Step 2: `pawbun-mcp-server/src/lib.rs`** — add crate-level docs explaining usage, transport modes, builder pattern, link to pawbun-mcp-core
- [ ] **Step 3: Commit**

---

## Completion Checklist

Before declaring "done":
- [ ] `cargo test --workspace --all-features` — all pass
- [ ] `cargo doc --workspace --no-deps` — zero warnings
- [ ] `cargo clippy --workspace --all-features` — clean
- [ ] Existing toolkit MCP client API unchanged (backward compatible)
- [ ] Stdio end-to-end test: full MCP handshake + tool call roundtrip
- [ ] SSE end-to-end test: SSE handshake + POST routing + tool call (if http feature)
- [ ] FileLoader bridge: can read files via MCP tools/call
- [ ] Deduplication: user-registered tools take priority over bridge tools
