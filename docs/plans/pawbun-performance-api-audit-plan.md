# Pawbun 0.3.0 性能、API 审计与文档 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Establish performance baselines, audit public API boundaries, add examples and docs, verify feature compatibility matrix, and evaluate dependency slimming across all Pawbun workspace crates.

**Architecture:** Horizontal quality pass across 5 crates. No new features; pure measurement, documentation, and API tightening. Benchmarks use Criterion; API audit uses docs.rs preview + manual review.

**Tech Stack:** Rust 1.75+, Criterion, cargo-doc, wiremock (for benchmarks), tokio, serde

**Spec:** `docs/specs/pawbun-performance-api-audit-spec.md`

---

## File Structure

```
New files:
  crates/pawbun-files/benches/loader.rs          — File loading + provider formatting benchmarks
  crates/pawbun-mcp-server/benches/handler.rs    — RequestHandler response time benchmarks
  crates/pawbun-toolkit/examples/basic_toolkit.rs     — Basic toolkit usage example
  crates/pawbun-toolkit/examples/custom_tool.rs       — Custom Tool trait implementation example
  crates/pawbun-files/examples/load_image.rs          — Image loading + provider formatting
  crates/pawbun-files/examples/batch_load.rs          — Batch loading with constraints
  crates/pawbun-mcp-server/examples/stdio_server.rs   — Stdio MCP server example
  crates/pawbun-mcp-server/examples/sse_server.rs     — SSE MCP server with CORS example
  crates/pawbun-mcp-core/examples/schema_convert.rs   — Schema bidirectional conversion
  crates/pawbun-mcp-core/examples/custom_transport.rs — In-memory Transport implementation
  docs/cookbook.md                                  — Cookbook style guides
  scripts/check-features.sh                         — Feature combination matrix script
  benches/README.md                                 — Benchmark report template
  docs/adr/0001-keep-image-crate.md                 — ADR if image-meta evaluation fails
  docs/adr/0002-keep-reqwest-async.md               — ADR if ureq evaluation fails

Modified files:
  crates/pawbun-toolkit/benches/toolkit.rs          — Expand existing benchmarks
  crates/pawbun-toolkit/src/lib.rs                  — Add #![deny(missing_docs)]
  crates/pawbun-toolkit/src/error.rs                — Add docs
  crates/pawbun-toolkit/src/toolkit.rs              — Add docs
  crates/pawbun-toolkit/src/tool.rs                 — Add docs
  crates/pawbun-toolkit/src/async_tool.rs           — Add docs
  crates/pawbun-toolkit/src/registry.rs             — Add docs
  crates/pawbun-toolkit/src/json_utils.rs           — Downgrade pub → pub(crate)
  crates/pawbun-toolkit/src/mcp/mod.rs              — Downgrade internal modules
  crates/pawbun-files/src/lib.rs                    — Add #![deny(missing_docs)]
  crates/pawbun-files/src/loader.rs                 — Add docs, downgrade internals
  crates/pawbun-files/src/provider.rs               — Add docs, downgrade internals
  crates/pawbun-files/Cargo.toml                    — Add [[bench]] entry
  crates/pawbun-mcp-core/src/lib.rs                 — Add #![deny(missing_docs)]
  crates/pawbun-mcp-core/src/protocol.rs            — Add docs
  crates/pawbun-mcp-core/src/transport.rs           — Add docs
  crates/pawbun-mcp-core/src/schema_convert.rs      — Add docs, downgrade internals
  crates/pawbun-mcp-server/src/lib.rs               — Add #![deny(missing_docs)]
  crates/pawbun-mcp-server/src/server.rs            — Add docs
  crates/pawbun-mcp-server/src/handler.rs           — Add docs
  crates/pawbun-mcp-server/src/capabilities.rs      — Add docs
  crates/pawbun-mcp-server/src/error.rs             — Add docs
  crates/pawbun-mcp-server/src/tool_bridge.rs       — Downgrade pub → pub(crate)
  crates/pawbun-mcp-server/src/transport/sse.rs     — Add docs, downgrade internals
  crates/pawbun-mcp-server/Cargo.toml               — Add [[bench]] entry
  Cargo.toml (workspace)                            — Add bench targets if needed
  README.md                                         — Update for 0.2.0 features + quickstart
```

---

## Phase 1: Benchmarks

### Task 1.1: Expand pawbun-toolkit benchmarks

**Files:**
- Modify: `crates/pawbun-toolkit/benches/toolkit.rs`

- [ ] **Step 1: Add lookup_1000 benchmark**

```rust
fn benchmark_registry_lookup_1000(c: &mut Criterion) {
    let mut kit = ToolKit::new();
    for i in 0..1000 {
        kit.register(Box::new(NamedNoOpTool(format!("tool_{}", i))));
    }
    c.bench_function("registry_lookup/1000", |b| {
        b.iter(|| {
            let _ = kit.get(black_box("tool_500"));
        })
    });
}
```

- [ ] **Step 2: Add descriptions benchmark**

```rust
fn benchmark_tool_descriptions(c: &mut Criterion) {
    let mut kit = ToolKit::new();
    for i in 0..100 {
        kit.register(Box::new(NamedNoOpTool(format!("tool_{}", i))));
    }
    c.bench_function("tool_descriptions/100", |b| {
        b.iter(|| {
            let _ = black_box(kit.descriptions());
        })
    });
}
```

- [ ] **Step 3: Add schema_build benchmark**

```rust
fn benchmark_schema_build(c: &mut Criterion) {
    let params = vec![
        ToolParameter {
            name: "url".into(),
            description: "URL to fetch".into(),
            required: true,
            schema: json!({"type": "string", "format": "uri"}),
        },
        ToolParameter {
            name: "max_length".into(),
            description: "Max length".into(),
            required: false,
            schema: json!({"type": "integer"}),
        },
    ];
    c.bench_function("schema_build/10_params", |b| {
        b.iter(|| {
            let schema = build_input_schema(black_box(&params));
            black_box(schema);
        })
    });
}
```

- [ ] **Step 4: Register new benchmarks in criterion_group**

```rust
criterion_group!(
    benches,
    benchmark_registry_lookup,
    benchmark_registry_lookup_1000,
    benchmark_tool_execution,
    benchmark_register,
    benchmark_tool_descriptions,
    benchmark_schema_build
);
```

- [ ] **Step 5: Verify benchmark compiles**

Run: `cargo bench -p pawbun-toolkit --no-run`
Expected: Compiles successfully

- [ ] **Step 6: Run benchmarks**

Run: `cargo bench -p pawbun-toolkit`
Expected: All 6 benchmarks run and report results

- [ ] **Step 7: Commit**

```bash
git add crates/pawbun-toolkit/benches/toolkit.rs
git commit -m "bench(toolkit): expand benchmarks with lookup_1000, descriptions, schema_build"
```

### Task 1.2: Create pawbun-files benchmarks

**Files:**
- Create: `crates/pawbun-files/benches/loader.rs`
- Modify: `crates/pawbun-files/Cargo.toml` (add [[bench]] and dev-deps)

- [ ] **Step 1: Add Criterion + tempfile to dev-dependencies**

In `crates/pawbun-files/Cargo.toml`:
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
tempfile = "3"
tokio = { version = "1", features = ["rt", "macros"] }

[[bench]]
name = "loader"
harness = false
```

- [ ] **Step 2: Write loader benchmark**

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pawbun_files::{DefaultFileLoader, FileLoader};
use std::io::Write;

fn benchmark_load_local(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("test.txt");
    std::fs::write(&path, "hello world").unwrap();
    
    let loader = DefaultFileLoader::new();
    c.bench_function("load_local/text", |b| {
        b.iter(|| {
            let _ = loader.load_file(black_box(path.to_str().unwrap()));
        })
    });
}

criterion_group!(benches, benchmark_load_local);
criterion_main!(benches);
```

- [ ] **Step 3: Verify and run**

Run: `cargo bench -p pawbun-files --no-run`
Run: `cargo bench -p pawbun-files`

- [ ] **Step 4: Commit**

```bash
git add crates/pawbun-files/benches/loader.rs crates/pawbun-files/Cargo.toml
git commit -m "bench(files): add file loading benchmark"
```

### Task 1.3: Create pawbun-mcp-server benchmarks

**Files:**
- Create: `crates/pawbun-mcp-server/benches/handler.rs`
- Modify: `crates/pawbun-mcp-server/Cargo.toml` (add [[bench]] and dev-deps)

- [ ] **Step 1: Add criterion to dev-dependencies**

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
tempfile = "3"

[[bench]]
name = "handler"
harness = false
```

- [ ] **Step 2: Write handler benchmark**

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pawbun_mcp_server::handler::RequestHandler;
use pawbun_mcp_core::protocol::*;
use pawbun_toolkit::ToolKit;
use serde_json::json;

fn benchmark_tools_list(c: &mut Criterion) {
    let mut toolkit = ToolKit::new();
    // Register 100 no-op tools
    for i in 0..100 {
        toolkit.register(Box::new(pawbun_toolkit::FileReadTool::default()));
    }
    
    let mut handler = RequestHandler::new(
        toolkit,
        ServerInfo { name: "bench".into(), version: "0.1.0".into() },
        json!({"tools": {}}),
        "2024-11-05".into(),
        None,
    );
    
    // Initialize first
    let init = JsonRpcRequest::new(
        1i64,
        "initialize",
        Some(json!({"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "bench", "version": "1.0"}})),
    );
    handler.handle(init);
    let notif = JsonRpcRequest::notification("notifications/initialized", None);
    handler.handle(notif);
    
    let req = JsonRpcRequest::new(2i64, "tools/list", None);
    c.bench_function("handler_tools_list/100", |b| {
        b.iter(|| {
            let resp = handler.handle(black_box(req.clone()));
            black_box(resp);
        })
    });
}

criterion_group!(benches, benchmark_tools_list);
criterion_main!(benches);
```

- [ ] **Step 3: Verify and run**

Run: `cargo bench -p pawbun-mcp-server --no-run`
Run: `cargo bench -p pawbun-mcp-server`

- [ ] **Step 4: Commit**

```bash
git add crates/pawbun-mcp-server/benches/handler.rs crates/pawbun-mcp-server/Cargo.toml
git commit -m "bench(mcp-server): add handler tools/list benchmark"
```

### Task 1.4: Create benchmark report

**Files:**
- Create: `benches/README.md`

- [ ] **Step 1: Write report template**

```markdown
# Pawbun Performance Benchmark Report

> Environment: [OS / CPU / Rust version]
> Command: `cargo bench --workspace`
> Date: [YYYY-MM-DD]

## pawbun-toolkit

| Benchmark | Time | Throughput | Target Met |
|-----------|------|------------|------------|
| registry_lookup/100 | [TBD] | — | [ ] |
| registry_lookup/1000 | [TBD] | — | [ ] |
| tool_execute_overhead | [TBD] | — | [ ] |
| tool_register | [TBD] | — | [ ] |
| tool_descriptions/100 | [TBD] | — | [ ] |
| schema_build/10_params | [TBD] | — | [ ] |

## pawbun-files

| Benchmark | Time | Throughput | Target Met |
|-----------|------|------------|------------|
| load_local/text | [TBD] | — | [ ] |

## pawbun-mcp-server

| Benchmark | Time | Throughput | Target Met |
|-----------|------|------------|------------|
| handler_tools_list/100 | [TBD] | — | [ ] |
```

- [ ] **Step 2: Run all benchmarks and fill results**

Run: `cargo bench --workspace`
Copy results into README.md

- [ ] **Step 3: Commit**

```bash
git add benches/README.md
git commit -m "docs(bench): add benchmark report template with initial results"
```

---

## Phase 2: pub API Audit

### Task 2.1: Enable deny(missing_docs) on pawbun-toolkit

**Files:**
- Modify: `crates/pawbun-toolkit/src/lib.rs`

- [ ] **Step 1: Add directive**

```rust
#![deny(missing_docs)]
```

- [ ] **Step 2: Fix all missing_docs errors**

Run: `cargo doc -p pawbun-toolkit --no-deps`
For each missing_docs error, add doc comment.

Key files to document:
- `src/error.rs` — ToolError variants
- `src/toolkit.rs` — ToolKit methods
- `src/tool.rs` — Tool trait and associated types
- `src/async_tool.rs` — AsyncTool, BlockingExecutor, TokioExecutor
- `src/registry.rs` — ToolExecutor, ToolRegistry, AsyncToolExecutor
- `src/types.rs` — ToolParameter, ToolResult
- `src/mcp/` — DynamicTool, transport module
- `src/tools/` — All tool structs

- [ ] **Step 3: Downgrade json_utils to pub(crate)**

In `src/lib.rs`:
```rust
pub(crate) mod json_utils;
```

In files that import `json_utils::parse`, change to `crate::json_utils::parse`.

- [ ] **Step 4: Verify**

Run: `cargo doc -p pawbun-toolkit --no-deps`
Run: `cargo clippy -p pawbun-toolkit --all-features -- -D warnings`
Expected: Zero warnings

- [ ] **Step 5: Commit**

```bash
git add crates/pawbun-toolkit/src/
git commit -m "docs(toolkit): enable deny(missing_docs), add docs, downgrade json_utils to pub(crate)"
```

### Task 2.2: Enable deny(missing_docs) on pawbun-files

**Files:**
- Modify: `crates/pawbun-files/src/lib.rs`

- [ ] **Step 1-4:** Same pattern as Task 2.1

Key files to document:
- `src/lib.rs` — crate-level docs
- `src/file.rs` — File, MediaType, MediaContent
- `src/loader.rs` — FileLoader, DefaultFileLoader
- `src/provider.rs` — Provider formats
- `src/constraints.rs` — FileConstraints

- [ ] **Step 5: Commit**

```bash
git commit -m "docs(files): enable deny(missing_docs), add docs to all public APIs"
```

### Task 2.3: Enable deny(missing_docs) on pawbun-mcp-core

**Files:**
- Modify: `crates/pawbun-mcp-core/src/lib.rs`

- [ ] **Step 1-4:** Same pattern

Key files:
- `src/protocol.rs` — JsonRpcRequest, JsonRpcResponse, ServerInfo, etc.
- `src/transport.rs` — Transport, TransportError, TransportConfig
- `src/schema_convert.rs` — schema_to_parameters, parameters_to_schema

- [ ] **Step 5: Commit**

```bash
git commit -m "docs(mcp-core): enable deny(missing_docs), add docs to protocol and transport types"
```

### Task 2.4: Enable deny(missing_docs) on pawbun-mcp-server

**Files:**
- Modify: `crates/pawbun-mcp-server/src/lib.rs`

- [ ] **Step 1-4:** Same pattern

Key files:
- `src/server.rs` — McpServer, McpServerBuilder
- `src/handler.rs` — RequestHandler
- `src/capabilities.rs` — ServerCapabilities, ToolsCapability, etc.
- `src/error.rs` — McpServerError
- `src/tool_bridge.rs` — Downgrade to pub(crate)

- [ ] **Step 5: Commit**

```bash
git commit -m "docs(mcp-server): enable deny(missing_docs), add docs, downgrade tool_bridge to pub(crate)"
```

---

## Phase 3: Examples

### Task 3.1: pawbun-toolkit examples

**Files:**
- Create: `crates/pawbun-toolkit/examples/basic_toolkit.rs`
- Create: `crates/pawbun-toolkit/examples/custom_tool.rs`
- Modify: `crates/pawbun-toolkit/Cargo.toml` (add [[example]] entries)

- [ ] **Step 1: Write basic_toolkit example**

```rust
//! Basic ToolKit usage example.
//!
//! Demonstrates creating a ToolKit, registering built-in tools, and executing them.

use pawbun_toolkit::{ToolKit, ToolRegistry, ToolExecutor, FileReadTool};

fn main() {
    let mut toolkit = ToolKit::new();
    toolkit.register(Box::new(FileReadTool::default()));

    println!("Registered tools: {}", toolkit.len());
    println!("Available tools:\n{}", toolkit.descriptions());

    // Execute file_read tool (will fail if README.md doesn't exist)
    match toolkit.execute("file_read", r#"{"path": "README.md"}"#) {
        Ok(result) => println!("Content preview: {}", &result.content[..result.content.len().min(200)]),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

- [ ] **Step 2: Write custom_tool example**

```rust
//! Custom Tool implementation example.
//!
//! Demonstrates implementing the Tool trait manually without macros.

use pawbun_toolkit::{Tool, ToolError, ToolParameter, ToolResult};
use serde_json::json;
use std::borrow::Cow;

#[derive(Debug)]
struct GreetTool;

impl Tool for GreetTool {
    fn name(&self) -> &str { "greet" }
    fn description(&self) -> &str { "Greet someone by name." }
    fn parameters(&self) -> Cow<'static, [ToolParameter]> {
        Cow::Owned(vec![
            ToolParameter {
                name: "name".into(),
                description: "Name to greet".into(),
                required: true,
                schema: json!({"type": "string"}),
            },
        ])
    }
    fn execute(&self, input: &str) -> Result<ToolResult, ToolError> {
        let parsed: serde_json::Value = serde_json::from_str(input)
            .map_err(|e| ToolError::invalid_input(format!("invalid JSON: {e}")))?;
        let name = parsed.get("name").and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::invalid_input("missing 'name' field"))?;
        Ok(ToolResult {
            success: true,
            content: format!("Hello, {name}!"),
            metadata: None,
            elapsed_ms: None,
        })
    }
}

fn main() {
    let mut toolkit = pawbun_toolkit::ToolKit::new();
    toolkit.register(Box::new(GreetTool));
    let result = toolkit.execute("greet", r#"{"name": "Pawbun"}"#).unwrap();
    println!("{}", result.content);
}
```

- [ ] **Step 3: Register examples in Cargo.toml**

```toml
[[example]]
name = "basic_toolkit"
path = "examples/basic_toolkit.rs"

[[example]]
name = "custom_tool"
path = "examples/custom_tool.rs"
```

- [ ] **Step 4: Verify**

Run: `cargo check --example basic_toolkit -p pawbun-toolkit`
Run: `cargo check --example custom_tool -p pawbun-toolkit`

- [ ] **Step 5: Commit**

```bash
git add crates/pawbun-toolkit/examples/
git commit -m "examples(toolkit): add basic_toolkit and custom_tool examples"
```

### Task 3.2: pawbun-files examples

**Files:**
- Create: `crates/pawbun-files/examples/load_image.rs`
- Create: `crates/pawbun-files/examples/batch_load.rs`
- Modify: `crates/pawbun-files/Cargo.toml`

- [ ] **Step 1-5:** Write, register, verify, commit

Pattern same as Task 3.1.

### Task 3.3: pawbun-mcp-server examples

**Files:**
- Create: `crates/pawbun-mcp-server/examples/stdio_server.rs`
- Create: `crates/pawbun-mcp-server/examples/sse_server.rs`
- Modify: `crates/pawbun-mcp-server/Cargo.toml`

- [ ] **Step 1-5:** Write, register, verify, commit

Note: `sse_server.rs` requires `http` feature.

### Task 3.4: pawbun-mcp-core examples

**Files:**
- Create: `crates/pawbun-mcp-core/examples/schema_convert.rs`
- Create: `crates/pawbun-mcp-core/examples/custom_transport.rs`
- Modify: `crates/pawbun-mcp-core/Cargo.toml`

- [ ] **Step 1-5:** Write, register, verify, commit

### Task 3.5: Create cookbook and update README

**Files:**
- Create: `docs/cookbook.md`
- Modify: `README.md`

- [ ] **Step 1: Write cookbook**

```markdown
# Pawbun Cookbook

## How to add a custom tool

```rust
use pawbun_toolkit::{Tool, ToolKit, ToolResult, ToolError, ToolParameter};
// Implement Tool trait, then:
let mut toolkit = ToolKit::new();
toolkit.register(Box::new(MyTool));
```

## How to configure an MCP server

```rust
use pawbun_mcp_server::McpServer;
let server = McpServer::builder("my-server", "0.1.0")
    .with_tools_capability()
    .request_timeout(60_000)
    .register_toolkit(toolkit)
    .build();
```

## How to safely load files

```rust
use pawbun_files::{DefaultFileLoader, FileConstraints};
let loader = DefaultFileLoader::with_constraints(
    FileConstraints::default()
        .max_size(10 * 1024 * 1024)
        .allow_types(&["image/png", "image/jpeg"]),
);
```

## How to bridge external APIs

See `crates/pawbun-toolkit/examples/openai_vision.rs` for a complete example.

## How to run benchmarks

```bash
cargo bench --workspace
```
```

- [ ] **Step 2: Update README.md**

Add 0.2.0 feature highlights and quickstart section.

- [ ] **Step 3: Commit**

```bash
git add docs/cookbook.md README.md
git commit -m "docs: add cookbook and update README for 0.2.0/0.3.0"
```

---

## Phase 4: Compatibility Matrix

### Task 4.1: Create feature check script

**Files:**
- Create: `scripts/check-features.sh`

- [ ] **Step 1: Write script**

```bash
#!/usr/bin/env bash
set -euo pipefail

FEATURES=(
  ""
  "http"
  "tokio"
  "csv"
  "jsonpath"
  "schemars"
  "tracing"
  "macros"
  "http,tokio"
  "http,tokio,csv,jsonpath,schemars,tracing,macros"
)

for feat in "${FEATURES[@]}"; do
  if [ -z "$feat" ]; then
    echo "=== checking: no default features ==="
    cargo check --workspace --no-default-features
  else
    echo "=== checking: $feat ==="
    cargo check --workspace --no-default-features --features "$feat"
  fi
done

echo "=== checking: all features ==="
cargo check --workspace --all-features

echo "All feature combinations passed!"
```

- [ ] **Step 2: Make executable and test**

Run: `chmod +x scripts/check-features.sh && ./scripts/check-features.sh`
Expected: May reveal compilation errors in some feature combinations — fix them.

- [ ] **Step 3: Fix compilation errors**

Common issues:
- Code gated behind wrong `#[cfg(feature = ...)]`
- Missing `#[cfg]` on imports
- Default feature assumptions broken

- [ ] **Step 4: Verify minimal dependency tree**

Run: `cargo tree -p pawbun-toolkit --no-default-features`
Run: `cargo tree -p pawbun-mcp-server --no-default-features`
Expected: Minimal trees, no unexpected heavy deps.

- [ ] **Step 5: Commit**

```bash
git add scripts/check-features.sh
git commit -m "ci: add feature combination matrix verification script"
```

---

## Phase 5: Dependency Slimming

### Task 5.1: Evaluate image-meta vs image

**Files:**
- Evaluate: `crates/pawbun-files/Cargo.toml`
- Evaluate: `crates/pawbun-files/src/loader.rs` (image usage)

- [ ] **Step 1: Check current image usage**

Run: `grep -rn "image::" crates/pawbun-files/src/`
Identify which `image` APIs are used.

- [ ] **Step 2: Check image-meta capabilities**

Run: `cargo add --dev image-meta` (temporary)
Check if `image-meta` supports:
- JPEG/PNG/WebP dimensions extraction
- MIME type detection

- [ ] **Step 3: Decision**

If `image-meta` covers ≥ 90% of current usage:
- Replace `image` with `image-meta`
- Update code
- Run tests

If not:
- Write ADR: `docs/adr/0001-keep-image-crate.md`

- [ ] **Step 4: Commit**

```bash
git commit -m "deps(files): [replace image with image-meta | document decision to keep image]"
```

### Task 5.2: Evaluate ureq for sync scenarios

**Files:**
- Evaluate: `crates/pawbun-toolkit/src/tools/web_fetch.rs`
- Evaluate: `crates/pawbun-toolkit/src/tools/web_search.rs`

- [ ] **Step 1-4:** Same evaluation pattern

Note: `WebFetchTool` and `WebSearchTool` are async-only (`AsyncTool`). `ureq` is sync-only. Likely decision: **keep reqwest** for async. Write ADR if evaluation confirms.

- [ ] **Step 5: Commit**

```bash
git commit -m "deps(toolkit): document decision to keep reqwest for async HTTP"
```

---

## Phase 6: Final Verification

### Task 6.1: Full workspace check

- [ ] **Step 1: Compile**

Run: `cargo check --workspace --all-features`
Expected: Zero errors

- [ ] **Step 2: Clippy**

Run: `cargo clippy --workspace --all-features -- -D warnings`
Expected: Zero warnings

- [ ] **Step 3: Documentation**

Run: `cargo doc --workspace --all-features`
Expected: Zero warnings (including missing_docs)

- [ ] **Step 4: Tests**

Run: `cargo test --workspace --all-features`
Expected: All tests pass

- [ ] **Step 5: Benchmarks**

Run: `cargo bench --workspace --no-run`
Expected: All benchmarks compile

- [ ] **Step 6: Examples**

Run: `cargo check --examples --workspace`
Expected: All examples compile

- [ ] **Step 7: Feature matrix**

Run: `./scripts/check-features.sh`
Expected: All combinations pass

- [ ] **Step 8: Commit version bump**

```bash
git add -A
git commit -m "release: Pawbun 0.3.0 — performance baselines, API audit, docs, compatibility"
```
