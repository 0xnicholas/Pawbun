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
  crates/pawbun-toolkit/src/json_utils.rs           — Verify already mod-private (no change)
  crates/pawbun-toolkit/src/mcp/mod.rs              — Verify visibility (McpSession stays pub)
  crates/pawbun-toolkit-macros/src/lib.rs           — Add #![deny(missing_docs)]
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
use pawbun_mcp_core::parameters_to_schema;

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
            let schema = parameters_to_schema(black_box(&params));
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

- [ ] **Step 1: Add Criterion + tempfile + wiremock to dev-dependencies**

In `crates/pawbun-files/Cargo.toml`:
```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }
tempfile = "3"
tokio = { version = "1", features = ["rt", "macros"] }
wiremock = "0.6"

[[bench]]
name = "loader"
harness = false
```

- [ ] **Step 2: Write loader benchmark**

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pawbun_files::{DefaultFileLoader, File, FileLoader, OpenAiFormat, ProviderFormat};

fn benchmark_load_local(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("test.txt");
    std::fs::write(&path, "hello world").unwrap();
    
    let loader = DefaultFileLoader::new();
    let file = File::from_path(&path);
    c.bench_function("load_local/text", |b| {
        b.iter(|| {
            let _ = loader.load(black_box(&file));
        })
    });
}

fn benchmark_provider_format(c: &mut Criterion) {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("test.txt");
    std::fs::write(&path, "hello world").unwrap();
    
    let loader = DefaultFileLoader::new();
    let file = File::from_path(&path);
    let loaded = loader.load(&file).unwrap();
    let formatter = OpenAiFormat;
    
    c.bench_function("provider_format/openai/text", |b| {
        b.iter(|| {
            let _ = formatter.format_content(black_box(&loaded.content));
        })
    });
}

criterion_group!(benches, benchmark_load_local, benchmark_provider_format);
criterion_main!(benches);
```

> **说明**：`load_url_mock` 基准涉及 wiremock 的启动/停止开销，不适合作为微基准测试。Spec 中保留该项作为探索项，但 Plan 中暂不在 Criterion bench 中实现，改为在集成测试中验证 URL 加载性能。

- [ ] **Step 3: Verify and run**

Run: `cargo bench -p pawbun-files --no-run`
Run: `cargo bench -p pawbun-files`

- [ ] **Step 4: Commit**

```bash
git add crates/pawbun-files/benches/loader.rs crates/pawbun-files/Cargo.toml
git commit -m "bench(files): add file loading and provider format benchmarks"
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
use pawbun_toolkit::{ToolKit, Tool, ToolError, ToolParameter, ToolResult};
use serde_json::json;
use std::borrow::Cow;

#[derive(Debug)]
struct EchoTool;

impl Tool for EchoTool {
    fn name(&self) -> &str { "echo" }
    fn description(&self) -> &str { "Echoes input back." }
    fn parameters(&self) -> Cow<'static, [ToolParameter]> {
        Cow::Owned(vec![
            ToolParameter {
                name: "message".into(),
                description: "Message to echo".into(),
                required: true,
                schema: json!({"type": "string"}),
            },
        ])
    }
    fn execute(&self, input: &str) -> Result<ToolResult, ToolError> {
        Ok(ToolResult {
            success: true,
            content: input.into(),
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
        ServerInfo { name: "bench".into(), version: "0.1.0".into() },
        json!({"tools": {}}),
        "2024-11-05".into(),
        None,
    )
}

fn benchmark_initialize(c: &mut Criterion) {
    c.bench_function("handler_initialize", |b| {
        b.iter(|| {
            let mut handler = make_handler();
            let init = JsonRpcRequest::new(
                1i64,
                "initialize",
                Some(json!({"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "bench", "version": "1.0"}})),
            );
            let resp = handler.handle(black_box(init));
            black_box(resp);
            let notif = JsonRpcRequest::notification("notifications/initialized", None);
            handler.handle(black_box(notif));
        })
    });
}

fn benchmark_tools_list(c: &mut Criterion) {
    let mut handler = make_handler();
    let init = JsonRpcRequest::new(
        1i64,
        "initialize",
        Some(json!({"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "bench", "version": "1.0"}})),
    );
    handler.handle(init);
    let notif = JsonRpcRequest::notification("notifications/initialized", None);
    handler.handle(notif);
    
    let req = JsonRpcRequest::new(2i64, "tools/list", None);
    c.bench_function("handler_tools_list/1", |b| {
        b.iter(|| {
            let resp = handler.handle(black_box(req.clone()));
            black_box(resp);
        })
    });
}

fn benchmark_tools_call(c: &mut Criterion) {
    let mut handler = make_handler();
    let init = JsonRpcRequest::new(
        1i64,
        "initialize",
        Some(json!({"protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": {"name": "bench", "version": "1.0"}})),
    );
    handler.handle(init);
    let notif = JsonRpcRequest::notification("notifications/initialized", None);
    handler.handle(notif);
    
    let req = JsonRpcRequest::new(
        2i64,
        "tools/call",
        Some(json!({"name": "echo", "arguments": {"message": "hello"}})),
    );
    c.bench_function("handler_tools_call", |b| {
        b.iter(|| {
            let resp = handler.handle(black_box(req.clone()));
            black_box(resp);
        })
    });
}

criterion_group!(benches, benchmark_initialize, benchmark_tools_list, benchmark_tools_call);
criterion_main!(benches);
```

- [ ] **Step 3: Verify and run**

Run: `cargo bench -p pawbun-mcp-server --no-run`
Run: `cargo bench -p pawbun-mcp-server`

- [ ] **Step 4: Commit**

```bash
git add crates/pawbun-mcp-server/benches/handler.rs crates/pawbun-mcp-server/Cargo.toml
git commit -m "bench(mcp-server): add handler initialize, tools/list and tools/call benchmarks"
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
| provider_format/openai/text | [TBD] | — | [ ] |

## pawbun-mcp-server

| Benchmark | Time | Throughput | Target Met |
|-----------|------|------------|------------|
| handler_initialize | [TBD] | — | [ ] |
| handler_tools_list/1 | [TBD] | — | [ ] |
| handler_tools_call | [TBD] | — | [ ] |
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

- [ ] **Step 3: Verify json_utils visibility**

确认 `src/lib.rs` 中：`mod json_utils;`（非 `pub`）。
经代码审查，当前已是正确限制，无需改动。

确认 `src/tools/mod.rs` 中：`pub(crate) mod url_utils;`、`pub(crate) mod path_utils;`。
经代码审查，当前已是正确限制，无需改动。

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

### Task 2.5: Enable deny(missing_docs) on pawbun-toolkit-macros

**Files:**
- Modify: `crates/pawbun-toolkit-macros/src/lib.rs`

- [ ] **Step 1: Add directive**

```rust
#![deny(missing_docs)]
```

- [ ] **Step 2: Fix all missing_docs errors**

Run: `cargo doc -p pawbun-toolkit-macros --no-deps`
For each missing_docs error, add doc comment.

Key items to document:
- `pawbun_tool` 过程宏的 doc comment — 说明所有可用属性参数（`name`, `description`, `parameters` 等）和生成的代码结构
- 宏内部辅助函数 — 确认为 `pub(crate)` 或非 `pub`，确保无意外暴露

- [ ] **Step 3: Verify macro visibility**

确认宏 crate 中没有 `pub` 的内部辅助函数（如 `parse_attributes`、`expand_tool_impl` 等）。
如有，降级为 `pub(crate)` 或 `fn`（模块内私有）。

- [ ] **Step 4: Verify**

Run: `cargo doc -p pawbun-toolkit-macros --no-deps`
Run: `cargo clippy -p pawbun-toolkit-macros --all-features -- -D warnings`
Expected: Zero warnings

- [ ] **Step 5: Commit**

```bash
git add crates/pawbun-toolkit-macros/src/
git commit -m "docs(toolkit-macros): enable deny(missing_docs), add macro docs, verify visibility"
```

---

## Phase 3: Examples

> **说明**：以下任务按 crate 创建示例。部分 crate 在 0.2.0 已有示例，本次任务在已有基础上补充缺口，确保每个 crate 总计 ≥ 2 个可运行示例。
>
> - `pawbun-toolkit` 已有：`docker_code_executor.rs`、`openai_vision.rs`、`openai_embedding.rs`
> - `pawbun-files` 已有：`basic_usage.rs`、`provider_switching.rs`、`constraints.rs`
> - `pawbun-mcp-server`、`pawbun-mcp-core`、`pawbun-toolkit-macros` 尚无示例，需各新建 2 个

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

CRATES=(
  "pawbun-toolkit"
  "pawbun-files"
  "pawbun-mcp-core"
  "pawbun-mcp-server"
)

# 对每个 crate 检查最小依赖集（no-default-features）
for crate in "${CRATES[@]}"; do
  echo "=== $crate: no default features ==="
  cargo check -p "$crate" --no-default-features
done

# 对 pawbun-toolkit 检查关键 feature 组合
echo "=== pawbun-toolkit: key feature combinations ==="
cargo check -p pawbun-toolkit --no-default-features --features http
cargo check -p pawbun-toolkit --no-default-features --features tokio
cargo check -p pawbun-toolkit --no-default-features --features csv
cargo check -p pawbun-toolkit --no-default-features --features jsonpath
cargo check -p pawbun-toolkit --no-default-features --features schemars
cargo check -p pawbun-toolkit --no-default-features --features tracing
cargo check -p pawbun-toolkit --no-default-features --features macros
cargo check -p pawbun-toolkit --no-default-features --features "http,tokio,csv,jsonpath,schemars,tracing,macros"

# 对 pawbun-files 检查关键 feature 组合
echo "=== pawbun-files: key feature combinations ==="
cargo check -p pawbun-files --no-default-features --features url-source
cargo check -p pawbun-files --no-default-features --features image-meta
cargo check -p pawbun-files --no-default-features --features "url-source,image-meta,tracing,tokio"

# workspace 全 feature 验证
echo "=== workspace: all features ==="
cargo check --workspace --all-features

echo "All feature combinations passed!"
```

> **说明**：由于 workspace 未定义统一 features，直接 `cargo check --workspace --no-default-features --features "http"` 会报错 `none of the selected packages contains these features`。脚本改为 per-crate 检查最小依赖集 + 关键 feature 组合 + workspace 全 feature 验证的组合策略。

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
