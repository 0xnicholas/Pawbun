# Pawbun 生态集成 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Pawbun 0.2.0 — ecosystem integration: adapter examples for placeholder tools, error chain enhancement, MCP server configurability, and SSE transport stability.

**Architecture:** Incremental improvements on 0.1.0 core. No new workspace crates; changes confined to `pawbun-toolkit`, `pawbun-mcp-server`, and new example files.

**Tech Stack:** Rust 1.75+, thiserror, serde, tokio, axum, reqwest, tower-http

**Spec:** `docs/specs/pawbun-ecosystem-integration-spec.md`

---

## File Structure

```
New files:
  crates/pawbun-toolkit/examples/
    docker_code_executor.rs       — DockerCodeExecutor adapter example
    openai_vision.rs              — OpenAiVisionTool adapter example
    openai_embedding.rs           — OpenAiEmbeddingTool adapter example

Modified files:
  crates/pawbun-toolkit/Cargo.toml
    — Add [[example]] entries for the 3 adapters (conditional on http feature for vision/embedding)
  crates/pawbun-toolkit/src/error.rs
    — Refactor ToolError variants: InvalidInput, ExecutionFailed, Serialization → struct variants with source
  crates/pawbun-toolkit/src/lib.rs
  crates/pawbun-toolkit/Cargo.toml (dev-dependencies)
    — Add tokio = { version = "1", features = ["rt-multi-thread", "macros"] } for examples
  crates/pawbun-mcp-server/Cargo.toml
    — Add tower-http = { version = "0.6", features = ["cors"], optional = true } under [dependencies]
    — Include tower-http in the "http" feature
  crates/pawbun-mcp-server/src/server.rs
    — Extend McpServerBuilder: protocol_version, capabilities methods, cors_origins, request_timeout, tool_timeout
  crates/pawbun-mcp-server/src/handler.rs
    — Accept protocol_version in RequestHandler::new, use it in handle_initialize
    — Add timeout wrapping in handle_call_tool
  crates/pawbun-mcp-server/src/lib.rs
    — Re-export ServerCapabilities, ToolsCapability, LoggingCapability, LogLevel, etc.
  crates/pawbun-mcp-server/src/error.rs
    — No changes (McpServerError already has #[from], spec says don't touch)
  crates/pawbun-mcp-server/src/transport/sse.rs
    — Make heartbeat interval/text configurable via SseServerConfig
    — Add session TTL cleanup, max_connections guard
  crates/pawbun-toolkit/src/mcp/transport.rs
    — Add SseTransport::with_retry(max_retries, initial_backoff_ms)
```

---

## Phase 1: ToolError Chain Enhancement (P1)

**Goal:** Convert `ToolError` tuple variants (`InvalidInput`, `ExecutionFailed`, `Serialization`) to struct variants with `#[source]`, while preserving backward-compatible shortcut constructors.

### Task 1.1: Refactor ToolError variants

**File:** `crates/pawbun-toolkit/src/error.rs`

- [ ] **Step 1:** Change `InvalidInput(String)` to struct variant:
  ```rust
  #[error("invalid input: {message}")]
  InvalidInput {
      message: String,
      #[source]
      source: Option<Box<dyn std::error::Error + Send + Sync>>,
  }
  ```
- [ ] **Step 2:** Change `ExecutionFailed(String)` to struct variant:
  ```rust
  #[error("execution failed: {message}")]
  ExecutionFailed {
      message: String,
      #[source]
      source: Option<Box<dyn std::error::Error + Send + Sync>>,
  }
  ```
- [ ] **Step 3:** Change `Serialization(String)` to struct variant:
  ```rust
  #[error("serialization error: {message}")]
  Serialization {
      message: String,
      #[source]
      source: Option<Box<dyn std::error::Error + Send + Sync>>,
  }
  ```
- [ ] **Step 4:** Keep `NotFound(String)`, `Timeout(u64)`, `Io { message, kind }` unchanged.
- [ ] **Step 5:** Add shortcut constructors on `impl ToolError`:
  ```rust
  pub fn invalid_input(msg: impl Into<String>) -> Self {
      Self::InvalidInput { message: msg.into(), source: None }
  }
  pub fn execution_failed(msg: impl Into<String>) -> Self {
      Self::ExecutionFailed { message: msg.into(), source: None }
  }
  ```
- [ ] **Step 6:** Implement `with_source`:
  ```rust
  pub fn with_source(self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
      match self {
          Self::InvalidInput { message, .. } => Self::InvalidInput {
              message, source: Some(Box::new(source)),
          },
          Self::ExecutionFailed { message, .. } => Self::ExecutionFailed {
              message, source: Some(Box::new(source)),
          },
          Self::Serialization { message, .. } => Self::Serialization {
              message, source: Some(Box::new(source)),
          },
          other => other,
      }
  }
  ```

### Task 1.2: Update all call sites

**Files:** `crates/pawbun-toolkit/src/tools/*.rs`, `crates/pawbun-toolkit/src/toolkit.rs`, `crates/pawbun-mcp-server/src/handler.rs`, etc.

- [ ] **Step 1:** `grep -rn "ToolError::InvalidInput("` — replace direct tuple construction with `ToolError::invalid_input(...)` where source is unavailable.
- [ ] **Step 2:** `grep -rn "ToolError::ExecutionFailed("` — same as above.
- [ ] **Step 3:** `grep -rn "ToolError::Serialization("` — same as above.
- [ ] **Step 4:** In adapter code paths (e.g., `web_fetch`, `web_search`), wrap underlying `reqwest` errors with `.map_err(|e| ToolError::execution_failed("...").with_source(e))` where appropriate.
- [ ] **Step 5:** In `file_read`/`file_write` tools, wrap `pawbun_files::LoadError` with `with_source`.

### Task 1.3: Verify compilation

- [ ] **Step 1:** `cargo check -p pawbun-toolkit --all-features`
- [ ] **Step 2:** `cargo test -p pawbun-toolkit --all-features`
- [ ] **Step 3:** `cargo clippy -p pawbun-toolkit --all-features -- -D warnings`

---

## Phase 2: MCP Server Configurability (P1)

**Goal:** Make `McpServerBuilder` configurable for protocol version, typed capabilities, CORS, and timeouts.

### Task 2.1: Add ServerCapabilities types

**File:** `crates/pawbun-mcp-server/src/lib.rs` (or new `crates/pawbun-mcp-server/src/capabilities.rs`)

- [ ] **Step 1:** Create `ServerCapabilities`:
  ```rust
  #[derive(Debug, Clone, Default, serde::Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct ServerCapabilities {
      #[serde(skip_serializing_if = "Option::is_none")]
      pub tools: Option<ToolsCapability>,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub logging: Option<LoggingCapability>,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub prompts: Option<PromptsCapability>,
      #[serde(skip_serializing_if = "Option::is_none")]
      pub resources: Option<ResourcesCapability>,
  }
  ```
- [ ] **Step 2:** Create sub-capability structs + LogLevel:
  ```rust
  #[derive(Debug, Clone, serde::Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct ToolsCapability { pub list_changed: bool }

  #[derive(Debug, Clone, serde::Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct LoggingCapability { pub level: LogLevel }

  #[derive(Debug, Clone, serde::Serialize)]
  #[serde(rename_all = "camelCase")]
  pub enum LogLevel { Debug, Info, Warn, Error }

  #[derive(Debug, Clone, serde::Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct PromptsCapability;

  #[derive(Debug, Clone, serde::Serialize)]
  #[serde(rename_all = "camelCase")]
  pub struct ResourcesCapability;
  ```
- [ ] **Step 3:** Re-export from `lib.rs`.

### Task 2.2: Extend McpServerBuilder

**File:** `crates/pawbun-mcp-server/src/server.rs`

- [ ] **Step 1:** Add new fields to `McpServerBuilder`:
  ```rust
  pub struct McpServerBuilder {
      // existing fields...
      protocol_version: String,
      capabilities: ServerCapabilities,  // replaces Value
      raw_capabilities: Option<Value>,   // backward compat fallback
      cors_origins: Vec<String>,  // always present; builder method gated by #[cfg(http)]
      request_timeout_ms: Option<u64>,
      tool_timeout_ms: Option<u64>,
  }
  ```
- [ ] **Step 2:** Initialize defaults in `McpServerBuilder::new`:
  - `protocol_version: "2024-11-05".into()`
  - `capabilities: ServerCapabilities::default()`
  - `raw_capabilities: None`
  - `cors_origins: Vec::new()`  // empty by default
  - `request_timeout_ms: Some(30_000)`
  - `tool_timeout_ms: None`
- [ ] **Step 3:** Implement new builder methods:
  - `pub fn protocol_version(mut self, version: impl Into<String>) -> Self`
  - `pub fn with_tools_capability(mut self) -> Self` → set `self.capabilities.tools = Some(ToolsCapability { list_changed: false })`
  - `pub fn with_logging_capability(mut self, level: LogLevel) -> Self`
  - `pub fn with_prompts_capability(mut self) -> Self`
  - `pub fn with_resources_capability(mut self) -> Self`
  - `pub fn request_timeout(mut self, ms: u64) -> Self`
  - `pub fn tool_timeout(mut self, ms: u64) -> Self`
  - `pub fn cors_origins(mut self, origins: Vec<String>) -> Self` (http feature)
  - Keep existing `pub fn capabilities(mut self, caps: Value) -> Self` → set `raw_capabilities = Some(caps)` for backward compat.
- [ ] **Step 4:** In `build()`, determine final capabilities:
  ```rust
  let caps = if let Some(raw) = self.raw_capabilities {
      raw  // user explicitly set raw Value, use it
  } else {
      serde_json::to_value(&self.capabilities).unwrap_or_else(|_| json!({"tools": {}}))
  };
  ```

### Task 2.3: Wire protocol_version through RequestHandler

**File:** `crates/pawbun-mcp-server/src/handler.rs`

- [ ] **Step 1:** Add `protocol_version: String` field to `RequestHandler`.
- [ ] **Step 2:** Update `RequestHandler::new` signature:
  ```rust
  pub fn new(
      toolkit: ToolKit,
      server_info: ServerInfo,
      capabilities: Value,
      protocol_version: String,
  ) -> Self
  ```
- [ ] **Step 3:** In `handle_initialize`, replace hardcoded `"2024-11-05"`:
  ```rust
  if params.protocol_version != self.protocol_version { ... }
  // ...
  protocol_version: self.protocol_version.clone(),
  ```
- [ ] **Step 4:** Update `server.rs` `run_loop` to pass `self.protocol_version` to `RequestHandler::new`.

### Task 2.4: Add timeout wrapping to handler

**File:** `crates/pawbun-mcp-server/src/handler.rs`

- [ ] **Step 1:** Add `request_timeout_ms: Option<u64>` and `tool_timeout_ms: Option<u64>` fields to `RequestHandler`.
- [ ] **Step 2:** In `RequestHandler::handle`, wrap the entire dispatch with `request_timeout`:
  - If `request_timeout_ms` is `Some`, wrap the `match req.method.as_str()` block:
    - SSE 场景：用 `tokio::time::timeout(Duration::from_millis(timeout), async { ... }).await`
    - stdio 场景：用 `crossbeam::channel` 或 `std::thread::spawn` + `recv_timeout`
    - 超时后返回 `JsonRpcResponse::error(req.id, -32001, "Request timeout")`
  - If `None`, dispatch normally.
- [ ] **Step 3:** In `handle_call_tool`, apply `tool_timeout`:
  - If `tool_timeout_ms` is set, pass it into `ToolKit` via `with_timeout` before calling `execute()`.
  - If `tool_timeout_ms` is `None`, use `ToolKit`'s own `default_timeout_ms`.
- [ ] **Step 4:** In `McpServerBuilder::build()`, if `tool_timeout_ms` is set, apply it to toolkit before constructing handler:
  ```rust
  if let Some(ms) = self.tool_timeout_ms {
      self.toolkit = self.toolkit.with_timeout(ms);
  }
  ```

### Task 2.5: Add CORS support to SSE transport

**File:** `crates/pawbun-mcp-server/Cargo.toml`

- [ ] **Step 1:** Add dependency:
  ```toml
  tower-http = { version = "0.6", features = ["cors"], optional = true }
  ```
- [ ] **Step 2:** Include in `http` feature:
  ```toml
  http = ["dep:tokio", "dep:axum", "dep:uuid", "dep:async-stream", "dep:futures", "dep:tower-http"]
  ```

**File:** `crates/pawbun-mcp-server/src/transport/sse.rs`

- [ ] **Step 3:** Accept `SseServerConfig` in `SseServerTransport::new` (or new constructor `new_with_config`).
- [ ] **Step 4:** Add CORS layer to axum Router:
  ```rust
  use tower_http::cors::{Any, CorsLayer};

  let origins: Vec<http::HeaderValue> = config.cors_origins
      .iter()
      .map(|o| o.parse().expect("invalid CORS origin"))
      .collect();
  let cors = CorsLayer::new()
      .allow_origin(origins)
      .allow_methods(config.cors_methods)
      .allow_headers(config.cors_headers);

  let app = Router::new()
      .route("/sse", get(sse_handler))
      .route("/message", post(message_handler))
      .layer(cors)
      .with_state(app_state);
  ```

### Task 2.6: Verify compilation

- [ ] **Step 1:** `cargo check -p pawbun-mcp-server --all-features`
- [ ] **Step 2:** `cargo test -p pawbun-mcp-server --all-features`
- [ ] **Step 3:** `cargo clippy -p pawbun-mcp-server --all-features -- -D warnings`

---

## Phase 3: SSE Transport Stability (P1)

**Goal:** Configurable heartbeat, session management with TTL, max connections, and client-side retry exposure.

### Task 3.1: Make heartbeat configurable

**File:** `crates/pawbun-mcp-server/src/transport/sse.rs`

- [ ] **Step 1:** Define or extend `SseServerConfig`:
  ```rust
  pub struct SseServerConfig {
      pub bind_addr: String,
      pub cors_origins: Vec<String>,
      pub cors_methods: Vec<String>,
      pub cors_headers: Vec<String>,
      pub heartbeat_interval_ms: u64,  // default 15000
      pub heartbeat_text: String,       // default "ping"
  }
  ```
- [ ] **Step 2:** Update `SseServerTransport::new` to accept `SseServerConfig`.
- [ ] **Step 3:** In `sse_handler`, pass config and use configurable heartbeat:
  ```rust
  Sse::new(stream).keep_alive(
      axum::response::sse::KeepAlive::new()
          .interval(Duration::from_millis(config.heartbeat_interval_ms))
          .text(&config.heartbeat_text),
  )
  ```

### Task 3.2: Add session TTL and max connections

**File:** `crates/pawbun-mcp-server/src/transport/sse.rs`

- [ ] **Step 1:** Add `max_connections: usize` and `session_ttl: Duration` to `SseServerConfig` (defaults: 100, 30s).
- [ ] **Step 2:** In `AppState`, add `created_at: Instant` per session (or new `SseSession` struct).
- [ ] **Step 3:** In `sse_handler`, before accepting connection:
  ```rust
  let sessions = state.sessions.read().await;
  if sessions.len() >= config.max_connections {
      // Return 503 Service Unavailable
      return Err((
          axum::http::StatusCode::SERVICE_UNAVAILABLE,
          "max connections reached",
      ));
  }
  ```
- [ ] **Step 4:** Implement GC task spawned in `SseServerTransport::new`:
  ```rust
  let gc_state = state.clone();
  let gc_config = config.clone();
  runtime.spawn(async move {
      let mut interval = tokio::time::interval(Duration::from_secs(10));
      loop {
          interval.tick().await;
          let mut sessions = gc_state.sessions.write().await;
          let now = Instant::now();
          sessions.retain(|_, session| now.duration_since(session.created_at) < gc_config.session_ttl);
      }
  });
  ```
- [ ] **Step 5:** Track `last_activity` on each session, update on every message send.

### Task 3.3: Expose client retry configuration

**File:** `crates/pawbun-toolkit/src/mcp/transport.rs` (SSE client transport)

- [ ] **Step 1:** Add fields to `SseTransport`:
  ```rust
  max_retries: u32,
  initial_backoff_ms: u64,
  ```
  Default: `max_retries = 5`, `initial_backoff_ms = 1000`.
- [ ] **Step 2:** Add builder method:
  ```rust
  pub fn with_retry(mut self, max_retries: u32, initial_backoff_ms: u64) -> Self {
      self.max_retries = max_retries;
      self.initial_backoff_ms = initial_backoff_ms;
      self
  }
  ```
- [ ] **Step 3:** In `sse_reader_loop`, replace hardcoded backoff with instance fields.

### Task 3.4: Verify compilation

- [ ] **Step 1:** `cargo check -p pawbun-mcp-server --all-features`
- [ ] **Step 2:** `cargo check -p pawbun-toolkit --all-features`
- [ ] **Step 3:** `cargo test -p pawbun-mcp-server --all-features`
- [ ] **Step 4:** `cargo test -p pawbun-toolkit --all-features`

---

## Phase 4: Adapter Examples (P0)

**Goal:** Provide working adapter examples for `CodeExecuteTool`, `VisionTool`, `EmbeddingTool`.

### Task 4.1: DockerCodeExecutor example

**File:** `crates/pawbun-toolkit/examples/docker_code_executor.rs`

- [ ] **Step 1:** Create `DockerCodeExecutor` struct with builder pattern.
- [ ] **Step 2:** Implement `AsyncTool`.
- [ ] **Step 3:** Implement Docker HTTP API calls (containers/create, start, attach, wait, kill, remove).
- [ ] **Step 4:** Enforce security: whitelist images, NetworkMode=none, ReadonlyRootfs, resource limits.
- [ ] **Step 5:** Add `#[tokio::main]` example `main()` demonstrating:
  ```rust
  let executor = DockerCodeExecutor::new("unix:///var/run/docker.sock")
      .with_allowed_images(vec!["python:3.12-slim".into()])
      .with_timeout(30_000);
  let mut toolkit = ToolKit::new();
  toolkit.register(Box::new(executor));
  let result = toolkit.execute("code_execute", r#"{"code": "print(1+1)", "language": "python"}"#);
  println!("{}", result.unwrap().content);
  ```

### Task 4.2: OpenAiVisionTool example

**File:** `crates/pawbun-toolkit/examples/openai_vision.rs`

- [ ] **Step 1:** Create `OpenAiVisionTool` struct (requires `http` feature).
- [ ] **Step 2:** Implement `AsyncTool::execute_async`:
  - Parse input JSON: `image`, `prompt`.
  - Detect if `image` is path or data URI.
  - If path: use `pawbun_files::DefaultFileLoader` to load and base64 encode.
  - Build OpenAI chat.completions request with `image_url` content.
  - POST and parse response.
- [ ] **Step 3:** Add example `main()` with API key from env var.

### Task 4.3: OpenAiEmbeddingTool example

**File:** `crates/pawbun-toolkit/examples/openai_embedding.rs`

- [ ] **Step 1:** Create `OpenAiEmbeddingTool` struct (requires `http` feature).
- [ ] **Step 2:** Implement `AsyncTool::execute_async`:
  - Parse input JSON: `text` (string or array), `model` (optional).
  - POST `/embeddings`.
  - Return embedding array as JSON string in `ToolResult.content`.
- [ ] **Step 3:** Add example `main()`.

### Task 4.4: Register examples in Cargo.toml

**File:** `crates/pawbun-toolkit/Cargo.toml`

- [ ] **Step 1:** Add example entries:
  ```toml
  [[example]]
  name = "docker_code_executor"
  path = "examples/docker_code_executor.rs"
  required-features = []

  [[example]]
  name = "openai_vision"
  path = "examples/openai_vision.rs"
  required-features = ["http"]

  [[example]]
  name = "openai_embedding"
  path = "examples/openai_embedding.rs"
  required-features = ["http"]
  ```
- [ ] **Step 2:** Add dev-dependencies if needed (e.g., `tokio` with full features for examples).

### Task 4.5: Verify examples compile

- [ ] **Step 1:** `cargo check --example docker_code_executor -p pawbun-toolkit`
- [ ] **Step 2:** `cargo check --example openai_vision -p pawbun-toolkit --features http`
- [ ] **Step 3:** `cargo check --example openai_embedding -p pawbun-toolkit --features http`

---

## Phase 5: Acceptance & Integration

**Goal:** Full workspace builds, all tests pass, zero clippy warnings, documentation complete.

### Task 5.1: Full workspace check

- [ ] **Step 1:** `cargo check --workspace --all-features` → zero errors
- [ ] **Step 2:** `cargo clippy --workspace --all-features -- -D warnings` → zero warnings
- [ ] **Step 3:** `cargo test --workspace --all-features` → all tests pass (176 existing + new)
- [ ] **Step 4:** `cargo test --workspace --all-features --doc` → all doc-tests pass

### Task 5.2: Documentation

- [ ] **Step 1:** Ensure all new public APIs have doc comments with `# Example` blocks.
- [ ] **Step 2:** Update `README.md` with 0.2.0 feature highlights (optional but recommended).
- [ ] **Step 3:** Update `VERSIONS.md` with 0.2.0 release notes.

### Task 5.3: Final review checklist

- [ ] `ToolError` shortcut constructors still work (backward compat).
- [ ] `McpServerBuilder::capabilities(Value)` still works (backward compat).
- [ ] `McpServerError` and `LoadError` untouched.
- [ ] New examples compile independently.
- [ ] No new dependencies added to default feature set (tower-http only under `http`).

---

## Dependency Changes Summary

| Crate | Add | Remove | Modify |
|---|---|---|---|
| `pawbun-mcp-server` | `tower-http` (optional, under `http` feature) | — | `http` feature to include `tower-http` |
| `pawbun-toolkit` | `tokio` (dev-dep) | — | Add `[[example]]` entries |

---

## Risk & Mitigation

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `ToolError` refactor breaks existing tests | Medium | High | Run full test suite after each file change; preserve shortcut constructors |
| `ServerCapabilities` serialization mismatch with MCP clients | Low | Medium | Add integration test round-tripping capabilities JSON |
| Docker example fails on systems without Docker daemon | High (for CI) | Low | Mark example as `#[ignore]` in CI or gate behind feature |
| tower-http version conflict with axum 0.7 | Low | High | Pin `tower-http = "0.6"` which is compatible with axum 0.7 |
