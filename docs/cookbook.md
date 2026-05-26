# Pawbun Cookbook

## How to add a custom tool

```rust
use pawbun_toolkit::{Tool, ToolKit, ToolResult, ToolError, ToolParameter};
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

let mut toolkit = ToolKit::new();
toolkit.register(Box::new(GreetTool));
```

## How to configure an MCP server

```rust
use pawbun_mcp_server::McpServer;
use pawbun_mcp_core::transport::ServerTransportConfig;
use pawbun_toolkit::{ToolKit, FileReadTool};
use pawbun_files::DefaultFileLoader;

let mut toolkit = ToolKit::new();
toolkit.register(Box::new(FileReadTool::default()));

let loader = DefaultFileLoader::with_base_dir("/app/data");

let server = McpServer::builder("my-server", "0.1.0")
    .with_tools_capability()
    .request_timeout(60_000)
    .register_toolkit(toolkit)
    .register_file_loader(loader)
    .build();

server.launch(ServerTransportConfig::Stdio).unwrap();
```

## How to safely load files

```rust
use pawbun_files::{DefaultFileLoader, FileConstraints, OverflowMode, MediaType, ImageFormat};

let loader = DefaultFileLoader::with_constraints(
    FileConstraints::default()
        .max_size_bytes(10 * 1024 * 1024)
        .allow_types(&[MediaType::Image(ImageFormat::Png), MediaType::Image(ImageFormat::Jpeg)])
        .overflow_mode(OverflowMode::Strict),
);
```

## How to bridge external APIs (OpenAI example)

See `crates/pawbun-toolkit/examples/openai_vision.rs` for a complete example.

## How to run benchmarks

```bash
cargo bench --workspace
```

Results are written to `benches/README.md`.
