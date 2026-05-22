# pawbun-mcp-server Specification

> Version: 0.1.0-draft
> Status: Design
> Date: 2026-05-22

---

## 1. 目标与范围

`pawbun-mcp-server` 将 Pawbun 生态的工具能力（`pawbun-toolkit` 的 ToolKit + `pawbun-files` 的 FileLoader）通过标准 [Model Context Protocol](https://modelcontextprotocol.io/) 暴露给任意 MCP 客户端，使外部 Agent（Pandaria 其他组件或第三方工具）能以协议化的方式发现和调用 Pawbun 能力。

### 1.1 核心目标

- 实现 MCP 协议的 **Server 端**，支持 `initialize`、`tools/list`、`tools/call` 三个核心方法。
- 同时支持 **stdio** 和 **SSE** 两种 MCP 标准传输方式。
- 提供 Builder 模式 API，调用方注册 `ToolKit` 和 `FileLoader` 后一键启动。
- 自动将 `FileLoader` 能力包装为 MCP Tool（`file_read`、`file_list`），调用方无需手动适配。
- 严格遵守 MCP 规范：初始化状态机、标准 JSON-RPC 错误码、capabilities 声明。
- 将现有 MCP 协议通用类型提取到独立 `pawbun-mcp-core` crate，供 client 和 server 共用。

### 1.2 非目标

- 不实现 MCP `resources/*`、`prompts/*` 等其他能力（Phase 1 仅 tools，后续可扩展）。
- 不实现 SAML/OAuth 等认证层（SSE 可选加 API key，但非 Phase 1）。
- 不修改 `pawbun-toolkit` 和 `pawbun-files` 的现有公共 API（向后兼容）。
- 不替代现有 MCP client 模块（`McpAdapter`/`McpSession`）——client 能力保留在 toolkit，但从 core 引用协议类型。

---

## 2. Crate 结构

### 2.1 新增 crate

```
Pawbun/
├── Cargo.toml                          # 新增 pawbun-mcp-core, pawbun-mcp-server
├── crates/
│   ├── pawbun-mcp-core/                ← 新建
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── protocol.rs             # JSON-RPC 2.0 + MCP 消息类型
│   │       ├── transport.rs            # Transport trait + TransportConfig + TransportError
│   │       └── schema_convert.rs       # schema ↔ ToolParameter 双向转换
│   │
│   ├── pawbun-mcp-server/              ← 新建
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── server.rs               # McpServer + McpServerBuilder
│   │       ├── handler.rs              # RequestHandler：initialize 状态机 + 方法路由
│   │       ├── error.rs                # McpServerError
│   │       ├── tool_bridge.rs          # FileLoader → Tool 自动包装
│   │       └── transport/
│   │           ├── mod.rs
│   │           ├── stdio.rs            # StdioServerTransport
│   │           └── sse.rs              # SseServerTransport（需 http feature）
│   │
│   ├── pawbun-toolkit/                 ← 修改
│   │   ├── Cargo.toml                  # 依赖 pawbun-mcp-core
│   │   └── src/
│   │       ├── lib.rs                  # pub use pawbun_mcp_core 重新导出 MCP 类型
│   │       └── mcp/                    # adapter.rs + dynamic_tool.rs 保留，引用 core
│   │
│   ├── pawbun-toolkit-macros/          ← 不变
│   │
│   └── pawbun-files/                   ← 不变
```

### 2.2 依赖关系

```
pawbun-mcp-core  （零外部依赖，仅 serde/serde_json + thiserror）
  ↑                      ↑
  │                      │
pawbun-toolkit      pawbun-mcp-server
  ↑                      │
  │              ┌───────┘
  │              ↓
pawbun-toolkit-macros   pawbun-files
```

- `pawbun-mcp-core` 不依赖任何 Pawbun crate。
- `pawbun-toolkit` 依赖 core（替代现有内联 MCP 模块）。
- `pawbun-mcp-server` 依赖 core + toolkit + files。

### 2.3 现有 MCP 模块迁移

| 源文件 (pawbun-toolkit/src/mcp/) | 去向 |
|---|---|
| `protocol.rs` | → `pawbun-mcp-core/src/protocol.rs` |
| `transport.rs`（`Transport` trait + `TransportConfig` + `TransportError`） | → `pawbun-mcp-core/src/transport.rs` |
| `transport.rs`（`StdioTransport` 客户端实现） | → `pawbun-mcp-core/src/transport/stdio_client.rs` |
| `transport.rs`（`SseTransport` 客户端实现） | → `pawbun-mcp-core/src/transport/sse_client.rs`（保留现有 SSE 握手逻辑，仅提取） |
| `schema_convert.rs` | → `pawbun-mcp-core/src/schema_convert.rs` |
| `adapter.rs` + `dynamic_tool.rs` | 保留在 `pawbun-toolkit/src/mcp/`，引用 `pawbun_mcp_core` |
| `mod.rs` | 重构，重新导出 core 的类型，对外 API 不变 |

向后兼容保证：`pawbun_toolkit::mcp::*` 的现有公共 API 保持不变，通过 `pub use` 重新导出 core 的类型。

---

## 3. pawbun-mcp-core 设计

### 3.1 依赖

```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "1"
```

无任何 Pawbun 依赖。

### 3.2 protocol.rs — JSON-RPC 2.0 + MCP 消息类型

从 `pawbun-toolkit/src/mcp/protocol.rs` 迁移，**类型定义完全不变**：

- `JsonRpcRequest`, `JsonRpcResponse`, `JsonRpcError`, `JsonRpcId`
- `InitializeParams`, `InitializeResult`, `ClientInfo`, `ServerInfo`
- `McpToolDesc`, `ListToolsResult`
- `CallToolParams`, `CallToolResult`, `ToolContent`

**新增**便捷构造器：

```rust
impl JsonRpcResponse {
    /// 构造成功响应
    pub fn ok(id: Option<JsonRpcId>, result: Value) -> Self;
    /// 构造带序列化结果的响应
    pub fn ok_result(id: Option<JsonRpcId>, result: impl Serialize) -> Self;
    /// 构造错误响应（使用标准 JSON-RPC 错误码）
    pub fn error(id: Option<JsonRpcId>, code: i32, message: impl Into<String>) -> Self;
}
```

### 3.3 transport.rs — Transport 抽象（服务端扩展）

从 `pawbun-toolkit/src/mcp/transport.rs` 迁移，**新增服务端 trait**，保留客户端 trait：

```rust
/// 客户端 Transport（现有逻辑不变）
pub trait Transport: Send + Sync {
    fn request(&mut self, req: JsonRpcRequest) -> Result<JsonRpcResponse, TransportError>;
    fn close(self: Box<Self>) -> Result<(), TransportError>;
}

/// 服务端 Transport（新增）
pub trait ServerTransport: Send {
    fn recv(&mut self) -> Result<JsonRpcRequest, TransportError>;
    fn send(&mut self, resp: JsonRpcResponse) -> Result<(), TransportError>;
    fn close(self: Box<Self>) -> Result<(), TransportError>;
}

/// 客户端传输配置（现有，不变）
pub enum TransportConfig { Stdio { command, args }, Sse { url } }

/// 服务端传输配置（新增）
pub enum ServerTransportConfig {
    /// 标准输入输出（子进程模式）
    Stdio,
    /// HTTP + SSE 服务端（需 http feature）
    Sse { bind_addr: String },
}
```

`TransportError` 类型定义不变，从 toolkit 迁移。

### 3.4 schema_convert.rs — 双向 Schema 转换

从 toolkit 迁移现有 `schema_to_parameters()`，**新增逆向函数**：

```rust
use pawbun_toolkit::ToolParameter;
use serde_json::Value;

/// MCP input_schema → Vec<ToolParameter>（client 端用，现有逻辑）
pub fn schema_to_parameters(schema: &Value) -> Vec<ToolParameter>;

/// Vec<ToolParameter> → MCP inputSchema（server 端用，新增）
///
/// 将 ToolKit 中注册的工具参数列表转换为标准的 JSON Schema object，
/// 用于 tools/list 响应中的 inputSchema 字段。
///
/// # Example
/// ```
/// use pawbun_mcp_core::schema_convert::parameters_to_schema;
/// use pawbun_toolkit::ToolParameter;
/// use serde_json::json;
///
/// let params = vec![
///     ToolParameter {
///         name: "path".into(),
///         description: "File path".into(),
///         required: true,
///         schema: json!({"type": "string"}),
///     },
///     ToolParameter {
///         name: "max_length".into(),
///         description: "Max chars".into(),
///         required: false,
///         schema: json!({"type": "integer"}),
///     },
/// ];
///
/// let schema = parameters_to_schema(&params);
/// assert_eq!(schema["type"], "object");
/// assert_eq!(schema["required"][0], "path");
/// assert_eq!(schema["properties"]["path"]["type"], "string");
/// ```
pub fn parameters_to_schema(params: &[ToolParameter]) -> Value;
```

**ToolParameter 迁移方案（详细步骤）**：

`ToolParameter` 是纯数据结构（name/description/required/schema），MCP 协议是其核心消费者，放在 core 中语义合理。迁移分三步：

**Step 1 — core 定义**：在 `pawbun-mcp-core/src/lib.rs` 中定义 `ToolParameter`（带 `Serialize`/`Deserialize`），并公开导出。若需保留 `schemars` feature（`ToolParameter::from_schema::<T>()`），在 core 中增加 `#[cfg(feature = "schemars")]` 条件编译，`schemars` 作为 optional dep。

**Step 2 — toolkit 重新导出**：`pawbun-toolkit` 删掉 `src/types.rs` 中旧的 `ToolParameter` 定义，改为 `pub use pawbun_mcp_core::ToolParameter;`。同时为 `schemars` feature 保留 `impl ToolParameter { pub fn from_schema<T: schemars::JsonSchema>(...) }`，实现体迁移到 core 或保留在 toolkit（通过 `pub use` 后的类型拓展）。所有现有代码（`tool.rs` 的 `Tool::parameters()` 返回 `Cow<'static, [ToolParameter]>`）**无需修改**。

**Step 3 — schema_convert 自引用**：`pawbun-mcp-core/src/schema_convert.rs` 中 `parameters_to_schema()` 和 `schema_to_parameters()` 直接引用 core 自身的 `ToolParameter`，零外部依赖。

---

## 4. pawbun-mcp-server 设计

### 4.1 依赖

```toml
[dependencies]
pawbun-mcp-core = { path = "../pawbun-mcp-core" }
pawbun-toolkit = { path = "../pawbun-toolkit" }
pawbun-files = { path = "../pawbun-files" }
serde_json = "1"
thiserror = "1"
tokio = { version = "1", features = ["rt", "rt-multi-thread", "io-util", "sync", "macros"], optional = true }
axum = { version = "0.7", optional = true }

[features]
default = ["http"]
http = ["dep:tokio", "dep:axum"]
```

### 4.2 server.rs — McpServer + Builder

```rust
use pawbun_mcp_core::transport::ServerTransportConfig;
use pawbun_toolkit::{Tool, ToolKit};
use pawbun_files::DefaultFileLoader;
use serde_json::Value;

/// MCP Server，持有所有待暴露的能力。
pub struct McpServer {
    toolkit: ToolKit,
    server_info: pawbun_mcp_core::protocol::ServerInfo,
    capabilities: Value,
}

/// McpServer 构造器。
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
/// // 启动 stdio server（阻塞）
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
    /// 创建 Builder，指定服务端名称和版本。
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            toolkit: ToolKit::new(),
            file_loader: None,
            server_name: name.into(),
            server_version: version.into(),
            capabilities: json!({"tools": {}}),
        }
    }

    /// 注册一个 ToolKit。可多次调用，工具合并。
    /// 同名工具后注册的覆盖先注册的。
    pub fn register_toolkit(mut self, mut toolkit: ToolKit) -> Self;

    /// 注册一个 FileLoader，自动包装为 file_read / file_list 工具。
    ///
    /// **去重规则**：若 ToolKit 中已存在同名工具（如用户先注册了自定义 `FileReadTool`），
    /// `register_file_loader` 不会覆盖——已有的保留，bridge tool 不再插入。
    /// 这意味着用户手动注册的工具**优先于**自动生成的 bridge tool。
    /// 若用户希望使用 bridge tool 替换现有工具，应先不注册同名自定义工具，或调用
    /// `register_toolkit` 之前不注册、之后直接用 bridge 的覆盖行为依赖调用顺序。
    pub fn register_file_loader(mut self, loader: DefaultFileLoader) -> Self;

    /// 注册单个工具。
    /// 若同名工具已存在，覆盖。
    pub fn register_tool(mut self, tool: Box<dyn Tool>) -> Self;

    /// 覆盖默认 capabilities（默认 `{"tools": {}}`）。
    pub fn capabilities(mut self, caps: Value) -> Self;

    /// 构建 McpServer。
    pub fn build(self) -> McpServer;
}

impl McpServer {
    /// 同步启动（Stdio）。阻塞当前线程直到 stdin 关闭。
    pub fn launch(self, config: ServerTransportConfig) -> Result<(), McpServerError>;

    /// 异步启动（SSE）。需 tokio runtime。
    #[cfg(feature = "http")]
    pub async fn launch_async(self, config: ServerTransportConfig) -> Result<(), McpServerError>;
}
```

### 4.3 handler.rs — 请求处理与状态机

```rust
use pawbun_mcp_core::protocol::*;
use pawbun_toolkit::ToolKit;

/// MCP 请求处理器，管理初始化状态机。
struct RequestHandler {
    toolkit: ToolKit,
    server_info: ServerInfo,
    capabilities: Value,
    initialized: bool,
}

impl RequestHandler {
    /// 处理单个 JSON-RPC 请求，返回响应。
    ///
    /// 状态机：
    /// 1. 未初始化时，仅接受 `initialize` 和 `notifications/initialized`。
    /// 2. 完成初始化后，接受 `tools/list` 和 `tools/call`。
    pub fn handle(&mut self, req: JsonRpcRequest) -> JsonRpcResponse {
        // 阶段守卫：非握手请求在未初始化时拒绝
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
            "notifications/initialized" => self.handle_initialized(req),
            "tools/list" => self.handle_list_tools(req),
            "tools/call" => self.handle_call_tool(req),
            _ => JsonRpcResponse::error(req.id, -32601, format!("Method not found: {}", req.method)),
        }
    }

    fn handle_initialize(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let params: InitializeParams = match serde_json::from_value(req.params.unwrap_or_default()) {
            Ok(p) => p,
            Err(e) => return JsonRpcResponse::error(req.id, -32602, format!("Invalid params: {e}")),
        };

        // 协商 protocol version
        if params.protocol_version != "2024-11-05" {
            return JsonRpcResponse::error(
                req.id,
                -32603,
                format!("Unsupported protocol version: {}", params.protocol_version),
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

    fn handle_initialized(&mut self, _req: JsonRpcRequest) -> JsonRpcResponse {
        self.initialized = true;
        // 规范要求 notification 不返回响应
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
                input_schema: Some(pawbun_mcp_core::schema_convert::parameters_to_schema(
                    &tool.parameters(),
                )),
            })
            .collect();

        JsonRpcResponse::ok_result(req.id, ListToolsResult { tools })
    }

    fn handle_call_tool(&self, req: JsonRpcRequest) -> JsonRpcResponse {
        let params: CallToolParams = match serde_json::from_value(req.params.unwrap_or_default()) {
            Ok(p) => p,
            Err(e) => return JsonRpcResponse::error(req.id, -32602, format!("Invalid params: {e}")),
        };

        let input = params
            .arguments
            .map(|v| v.to_string())
            .unwrap_or_default();

        match self.toolkit.execute(&params.name, &input) {
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
                        (-32602, format!("Tool not found: {}", e))
                    }
                    _ => (-32603, e.to_string()),
                };
                JsonRpcResponse::error(req.id, code, msg)
            }
        }
    }
}
```

### 4.4 错误码对齐

| 场景 | 错误码 | 来源 |
|---|---|---|
| 未初始化收到非握手请求 | `-32002` | MCP 扩展 |
| `tools/call` 工具未找到 | `-32602` | JSON-RPC Invalid params |
| 参数解析失败 | `-32602` | JSON-RPC Invalid params |
| 未知 method | `-32601` | JSON-RPC Method not found |
| 工具执行内部失败 | `-32603` | JSON-RPC Internal error |
| initialize 协议版本不匹配 | `-32603` | JSON-RPC Internal error |
| 传输层错误 | `-32603` | JSON-RPC Internal error |

### 4.5 tool_bridge.rs — FileLoader → Tool 自动包装

```rust
use pawbun_files::DefaultFileLoader;
use pawbun_toolkit::{Tool, ToolParameter, ToolResult, ToolError};
use std::borrow::Cow;

/// 将 FileLoader 读取能力包装为 file_read Tool。
pub(crate) struct FileReadBridgeTool {
    loader: DefaultFileLoader,
}

impl Tool for FileReadBridgeTool {
    fn name(&self) -> &str { "file_read" }
    fn description(&self) -> &str {
        "Read contents of a file. Supports text, images, PDFs, audio, and video."
    }
    fn parameters(&self) -> Cow<'static, [ToolParameter]> { /* path: string (required) */ }
    fn execute(&self, input: &str) -> Result<ToolResult, ToolError> {
        let file = pawbun_files::File::from_path(/* parsed path */);
        let loaded = self.loader.load(&file)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;
        let content_json = serde_json::to_string(&loaded.content)
            .map_err(|e| ToolError::Serialization(e.to_string()))?;
        Ok(ToolResult {
            success: true,
            content: content_json,
            metadata: None,
            elapsed_ms: None,
        })
    }
}

/// 将 FileLoader 元数据能力包装为 file_list Tool。
pub(crate) struct FileListBridgeTool {
    loader: DefaultFileLoader,
}

// 类似 file_read，但使用 loader.metadata() 而非 load()
```

`register_file_loader(loader)` 时自动将这两个 Tool 注册到内部 ToolKit。

**去重行为**：插入前检查 ToolKit 中是否已存在同名工具。若已有（如用户先调 `register_toolkit` 注册了自定义 `FileReadTool`），则跳过该 bridge tool，保留用户提供的版本。这确保用户手动注册的工具优先于自动生成的 bridge tool。

### 4.6 error.rs

```rust
use pawbun_mcp_core::transport::TransportError;
use pawbun_toolkit::ToolError;
use pawbun_files::LoadError;

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

---

## 5. 传输层实现

### 5.1 StdioServerTransport

```rust
// pawbun-mcp-server/src/transport/stdio.rs

use std::io::{BufRead, BufReader, Write};
use pawbun_mcp_core::transport::{ServerTransport, TransportError};
use pawbun_mcp_core::protocol::{JsonRpcRequest, JsonRpcResponse};

pub struct StdioServerTransport {
    stdin: BufReader<std::io::Stdin>,
    stdout: std::io::Stdout,
}

impl StdioServerTransport {
    pub fn new() -> Self;
}

impl ServerTransport for StdioServerTransport {
    fn recv(&mut self) -> Result<JsonRpcRequest, TransportError> {
        let mut line = String::new();
        let n = self.stdin.read_line(&mut line)
            .map_err(|e| TransportError::Io { ... })?;
        if n == 0 {
            return Err(TransportError::UnexpectedEof);
        }
        serde_json::from_str(&line)
            .map_err(|e| TransportError::Serialization(e.to_string()))
    }

    fn send(&mut self, resp: JsonRpcResponse) -> Result<(), TransportError> {
        // MCP 规范：notification（id: null）不期望响应。
        // handler 内部对 notifications/initialized 返回的响应为 {jsonrpc, id: null, result: null, error: null}，
        // transport 层检测到此模式后不向 stdout 输出任何内容，避免客户端误解析。
        let is_empty_notification_response =
            resp.id.is_none() && resp.result.is_none() && resp.error.is_none();
        if is_empty_notification_response {
            return Ok(());
        }
        let line = serde_json::to_string(&resp)
            .map_err(|e| TransportError::Serialization(e.to_string()))?;
        writeln!(self.stdout, "{}", line)
            .map_err(|e| TransportError::Io { ... })?;
        self.stdout.flush()
            .map_err(|e| TransportError::Io { ... })
    }

    fn close(self: Box<Self>) -> Result<(), TransportError> {
        Ok(()) // stdio 无需特殊清理
    }
}
```

### 5.2 SseServerTransport

```rust
// pawbun-mcp-server/src/transport/sse.rs
// 仅 #[cfg(feature = "http")] 时编译

use axum::{Router, routing::get, routing::post, extract::State};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use std::collections::HashMap;

pub struct SseServerTransport {
    // 内部持有 axum router 和 session 管理器
    sessions: Arc<RwLock<HashMap<String, Session>>>,
    bind_addr: String,
}

struct Session {
    /// 向该 session 的 SSE 通道发送 JSON-RPC 响应的 sender
    response_tx: mpsc::UnboundedSender<JsonRpcResponse>,
}
```

**SSE 握手流程**（参考现有 `SseTransport` 客户端实现但角色反转）：

```
Client                          SseServerTransport
  │                                 │
  │──── GET /sse ───────────────────→│  建立 SSE 长连接
  │                                 │  生成 sessionId
  │←─── event: endpoint ────────────│  data: /message?sessionId=xxx
  │                                 │
  │──── POST /message?sessionId=xxx →│  接收 JSON-RPC request
  │     {jsonrpc, id, method, ...}  │
  │                                 │  → 交给 handler 处理
  │                                 │  → 通过 session response_tx 发回
  │←─── event: message ────────────│  data: {jsonrpc, id, result}
  │     (SSE 通道返回)              │
```

**关键实现细节**：
- 复用现有 `SseParser` 的模式但方向相反——服务端 **发送** SSE，而非接收。
- 多 session 并发安全：`RwLock<HashMap>` + `tokio::mpsc` 通道。
- 服务端负责 session 超时清理（如 60s 无活动断开）。
- Notification 请求（`id: null`）：POST 返回 202 Accepted，不通过 SSE 返回响应。

---

## 6. 启动流程

```rust
impl McpServer {
    pub fn launch(self, config: ServerTransportConfig) -> Result<(), McpServerError> {
        match config {
            ServerTransportConfig::Stdio => {
                let transport = StdioServerTransport::new();
                self.run_loop(Box::new(transport))
            }
            ServerTransportConfig::Sse { bind_addr } => {
                #[cfg(feature = "http")]
                {
                    let transport = SseServerTransport::new(&bind_addr)?;
                    self.run_loop(Box::new(transport))
                }
                #[cfg(not(feature = "http"))]
                Err(McpServerError::Bind("SSE requires 'http' feature".into()))
            }
        }
    }

    fn run_loop(mut self, mut transport: Box<dyn ServerTransport>) -> Result<(), McpServerError> {
        let mut handler = RequestHandler::new(self.toolkit, self.server_info, self.capabilities);

        loop {
            let req = match transport.recv() {
                Ok(req) => req,
                Err(TransportError::UnexpectedEof) => break,
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

`SseServerTransport` 内部有自己的 event loop（axum serve），因此 `run_loop` 对于 SSE 模式的行为是：启动 HTTP server，阻塞直至 shutdown。

---

## 7. Feature Flags

### pawbun-mcp-core
无 feature flags（最小依赖，始终全量编译）。

### pawbun-mcp-server

```toml
[features]
default = ["http"]
http = ["dep:tokio", "dep:axum"]
```

`http` 默认启用。若调用方仅需 stdio（如嵌入式场景、最小依赖），可 `default-features = false`。

### pawbun-toolkit（修改）

`macros` feature 不做改动。`pawbun_mcp_core` 是**必须依赖**（不再通过 feature 控制）。

---

## 8. 测试策略

### 8.1 单元测试

| 测试对象 | 文件 | 覆盖要点 |
|---|---|---|
| `parameters_to_schema` | `pawbun-mcp-core/tests/schema_tests.rs` | 空参数、单参数、多参数（含 required/optional 混合）、嵌套 schema |
| `RequestHandler::handle` | `pawbun-mcp-server/tests/handler_tests.rs` | initialize 正确/错误版本、未初始化拒绝工具调用、tools/list 返回正确结构、tools/call 成功/失败、未知 method |
| `FileReadBridgeTool` | `pawbun-mcp-server/tests/tool_bridge_tests.rs` | 正常读取、路径穿越、类型冲突、返回 MediaContent 序列化 |
| `FileListBridgeTool` | 同上 | 返回文件元数据列表 |
| `StdioServerTransport` | `pawbun-mcp-server/tests/stdio_transport_tests.rs` | 发送/接收 JSON-RPC、notification 不响应 |

### 8.2 集成测试

| 测试场景 | 方式 | 覆盖 |
|---|---|---|
| **Stdio 端到端** | 启动当前 binary 作为子进程，通过 stdin 发 initialize + tools/list + tools/call，断言 stdout 输出 | 完整 MCP 握手 + 工具调用往返 |
| **SSE 端到端** | `#[tokio::test]`，启动 `McpServer::launch_async` 到随机端口，HTTP client 完成握手 + 工具调用 | 多 session、POST 路由、SSE 响应解析 |
| **向后兼容** | `pawbun-toolkit` 现有测试全部通过，`pawbun_toolkit::mcp::*` API 保持不变 | 迁移无破坏 |

### 8.3 Mock 策略

- `pawbun-toolkit` 的现有 mock transport 测试（`adapter.rs` 中 `MockTransport`）迁移到 core 的测试。
- `handler.rs` 测试直接构造 `ToolKit` + 假 `Tool` 实现，不依赖真实文件系统。
- `tool_bridge.rs` 测试使用 `tempfile` 创建临时文件。

---

## 9. 迁移计划

### Phase 1：提取 pawbun-mcp-core

1. 新建 `crates/pawbun-mcp-core/`。
2. 迁移 `protocol.rs`、`transport.rs`（仅 trait + 错误类型 + config）、`schema_convert.rs`。
3. 在 core 的 `lib.rs` 中定义 `ToolParameter`，添加 `parameters_to_schema()`。
4. `pawbun-toolkit` 依赖 core，通过 `pub use` 保持公共 API 不变。
5. 运行 `cargo test --workspace` 确保无破坏。

### Phase 2：实现 pawbun-mcp-server

1. 新建 `crates/pawbun-mcp-server/`。
2. 实现 `server.rs`、`handler.rs`、`error.rs`、`tool_bridge.rs`。
3. 实现 `transport/stdio.rs`（无额外依赖）。
4. 实现 `transport/sse.rs`（需 `http` feature）。
5. 编写所有测试。

### Phase 3：迁移 MCP client 模块

1. `pawbun-toolkit/src/mcp/transport.rs` 中的 `StdioTransport` 和 `SseTransport` 客户端实现迁移到 `pawbun-mcp-core`。
2. `pawbun-toolkit/src/mcp/adapter.rs` 和 `dynamic_tool.rs` 保留在 toolkit，引用 core。
3. 验证所有现有测试通过。

---

## 10. 路线图

| 阶段 | 内容 |
|---|---|
| **Phase 1 (core)** | pawbun-mcp-core crate：protocol + transport trait + schema_convert 双向 + ToolParameter 迁移 |
| **Phase 2 (server)** | pawbun-mcp-server crate：McpServer + Builder + handler 状态机 + stdio transport |
| **Phase 3 (sse)** | SseServerTransport（http feature） |
| **Phase 4 (bridge)** | FileLoader → Tool 自动包装 + 集成测试 |
| **Phase 5 (migrate)** | 现有 MCP client 模块迁移到引用 core |
| **Phase 6 (doc)** | 示例代码 + README 更新 + 与 Pandaria 的集成指南 |

---

## 11. 附录：与 MCP 规范的合规性清单

| 规范要求 | 实现 | 状态 |
|---|---|---|
| JSON-RPC 2.0 消息格式 | `JsonRpcRequest` / `JsonRpcResponse` | ✅ |
| `initialize` 握手 | handler 返回 `InitializeResult` + capabilities | ✅ |
| `notifications/initialized` | 通知不返回响应 | ✅ |
| 初始化前拒绝非握手请求 | 状态机 `-32002` | ✅ |
| `tools/list` 返回 `inputSchema` | `parameters_to_schema()` | ✅ |
| `tools/call` 返回 `CallToolResult` | `ToolResult → CallToolResult` 映射 | ✅ |
| `tools/call` 错误用 `isError: true` | 映射自 `ToolResult::success` | ✅ |
| 标准 JSON-RPC 错误码 | `-32601` / `-32602` / `-32603` / `-32002` | ✅ |
| stdio 传输（换行分隔 JSON） | `StdioServerTransport` | ✅ |
| SSE 传输（endpoint 事件 + POST） | `SseServerTransport` | ✅ |
| Nested `$ref` in inputSchema | 不做额外解析，透传 ToolParameter::schema（用户自理） | ⚠️ 暂不处理 |
| `resources/*` / `prompts/*` | Phase 1 不实现 | ❌ 未来扩展 |
