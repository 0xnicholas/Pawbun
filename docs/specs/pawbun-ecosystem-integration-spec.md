# Pawbun 生态集成 Specification

> Version: 0.2.0-draft
> Status: Design
> Date: 2026-05-25
> Scope: pawbun-toolkit + pawbun-mcp-server

---

## 1. 目标与范围

0.1.0 已完成了 Pawbun 的核心基础设施：`Tool` trait、`ToolKit` 注册表、多模态文件处理、MCP 协议核心、MCP 服务器。但 `CodeExecuteTool`、`VisionTool`、`EmbeddingTool` 三个内置工具仅为**接口占位**——它们定义了标准输入输出契约，但 `execute` 方法直接返回错误，无法实际运行。

### 1.1 核心目标

- **将占位工具转为可运行**：为 `CodeExecuteTool`、`VisionTool`、`EmbeddingTool` 提供官方适配器示例，演示如何桥接外部服务。
- **适配器即文档**：示例代码本身即为集成指南，开发者可复制、修改、扩展。
- **错误链式上下文**：增强 `ToolError` 的可调试性，支持 `source()` 追溯根因；`McpServerError` 已通过 `#[from]` 实现链式错误，无需改动。
- **MCP 服务器可配置化**：`McpServerBuilder` 支持自定义协议版本、capabilities、SSE CORS 等。
- **SSE 传输稳定性**：增加重连、心跳、连接池管理，使 SSE 从"能跑"到"可靠"。

### 1.2 非目标

- 不引入重量级运行时依赖（如完整 Docker SDK、OpenAI 官方 Rust SDK）到 workspace 核心 crate。
- 适配器示例**不直接并入** `pawbun-toolkit` 的默认 feature——它们作为独立示例或可选子 crate 存在。
- 不实现通用 LLM 客户端（由 Pandaria 其他模块负责）；适配器仅演示如何为特定工具桥接特定 API。
- 不修改 `pawbun-toolkit-macros`（向后兼容）。
- `pawbun-files` 的 `LoadError` 和 `pawbun-mcp-server` 的 `McpServerError` 在 0.2.0 中不改动。

---

## 2. 架构概述

0.2.0 的改动分布在三个层次：

```
┌──────────────────────────────────────────────────────────────┐
│  项目外：外部服务                                               │
│  - Docker Daemon (HTTP API / CLI)                             │
│  - OpenAI API / Anthropic API / FastEmbed                     │
│  - Pandaria LLM Client (未来统一入口)                          │
└───────────────────────────┬──────────────────────────────────┘
                            │ 适配层
┌───────────────────────────▼──────────────────────────────────┐
│  生态集成层（本 Spec 新增）                                    │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  适配器示例 (examples/ 或独立子 crate)                  │ │
│  │  - DockerCodeExecutor：CodeExecuteTool 的 Docker 实现   │ │
│  │  - OpenAiVisionTool：VisionTool 的 OpenAI API 实现       │ │
│  │  - OpenAiEmbeddingTool：EmbeddingTool 的 OpenAI 实现     │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  错误增强                                               │ │
│  │  - ToolError → #[source] 链                             │ │
│  │  - McpServerError（已有 #[from]，已支持 source()）       │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  MCP 服务器增强                                         │ │
│  │  - McpServerBuilder::protocol_version()                │ │
│  │  - McpServerBuilder::capabilities()                    │ │
│  │  - McpServerBuilder::cors()                            │ │
│  │  - SSE 重连 + 心跳 + 连接池                            │ │
│  └─────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
                            │
┌───────────────────────────▼──────────────────────────────────┐
│  0.1.0 核心层（不变）                                         │
│  pawbun-toolkit / pawbun-files / pawbun-mcp-core /           │
│  pawbun-mcp-server / pawbun-toolkit-macros                   │
└──────────────────────────────────────────────────────────────┘
```

---

## 3. 占位工具适配器

### 3.1 设计原则

| 原则 | 说明 |
|---|---|
| **trait 优先** | 适配器必须实现 `Tool` 或 `AsyncTool` trait，与现有 `ToolKit` 无缝兼容。 |
| **配置外部化** | API key、endpoint、模型名等通过构造函数传入，不硬编码。 |
| **错误透明** | 外部服务错误（HTTP 4xx/5xx、JSON 解析失败）必须映射为 `ToolError`，保留原始错误上下文。 |
| **最小依赖** | 适配器示例可用 `reqwest` 手写 HTTP 请求，不强制绑定大型 SDK。 |
| **Feature 可选** | 若适配器并入 crate，必须放在独立 feature 下（如 `docker-sandbox`）。 |
| **同名替换** | 适配器使用与占位工具完全相同的 `name()` 返回值，注册到 `ToolKit` 时直接覆盖占位工具，实现无缝替换。 |
| **密钥安全** | API key 的安全存储（密钥管理、环境变量、密钥库）由调用方负责，适配器示例仅使用普通 `String` 演示接口。 |

### 3.2 CodeExecuteTool → Docker 沙箱适配器

```rust
use pawbun_toolkit::{Tool, ToolResult, ToolError, CodeExecuteTool};
use serde_json::json;

/// Docker 沙箱代码执行器。
///
/// 通过调用 Docker Daemon HTTP API（`/containers/create` + `/containers/{id}/start`）
/// 在隔离容器中执行用户代码，支持超时强制终止。
///
/// # 安全约束
/// - 仅允许预定义镜像白名单（如 `python:3.12-slim`）
/// - 限制容器资源（CPU、内存、无网络）
/// - 超时后强制 `docker kill`
/// - 禁止挂载宿主机敏感目录（仅挂载临时只读/可写目录）
pub struct DockerCodeExecutor {
    docker_host: String,
    allowed_images: Vec<String>,
    default_timeout_ms: u64,
    memory_limit_mb: u64,
    // 内部 HTTP client (reqwest::Client)
}

impl DockerCodeExecutor {
    pub fn new(docker_host: impl Into<String>) -> Self;
    pub fn with_allowed_images(mut self, images: Vec<String>) -> Self;
    pub fn with_timeout(mut self, ms: u64) -> Self;
    pub fn with_memory_limit(mut self, mb: u64) -> Self;
}

#[async_trait::async_trait]
impl AsyncTool for DockerCodeExecutor {
    fn name(&self) -> &str { "code_execute" }  // 与占位工具同名，注册时直接替换
    fn description(&self) -> &str { "Execute code in a Docker sandbox." }
    // parameters 与 CodeExecuteTool 完全一致，保持 LLM 契约稳定

    async fn execute_async(&self, input: &str) -> Result<ToolResult, ToolError> {
        // 1. 解析 input JSON：code, language, timeout_ms
        // 2. 校验 language → 镜像映射（如 python → python:3.12-slim）
        // 3. 校验镜像是否在 allowed_images 白名单中
        // 4. 创建临时目录，写入代码文件
        // 5. POST /containers/create (HostConfig: NetworkMode=none, Memory=limit)
        // 6. POST /containers/{id}/start
        // 7. 等待输出（带超时），超时则 POST /containers/{id}/kill
        // 8. 读取 stdout/stderr，清理容器和临时目录
        // 9. 返回 ToolResult
    }
}
```

**关键安全决策**：
- 默认 `NetworkMode = "none"`，禁止容器访问网络（防 SSRF 逃逸）。
- `ReadonlyRootfs = true`，代码通过临时卷挂载。
- 超时强制 `kill` 而非 `stop`，防止恶意代码忽略 SIGTERM。

### 3.3 VisionTool → OpenAI 视觉适配器

```rust
use pawbun_toolkit::{Tool, ToolResult, ToolError, AsyncTool, VisionTool};
use serde_json::json;

/// OpenAI GPT-4V / GPT-4o 视觉分析适配器。
///
/// 输入图片支持两种方式：
/// - Base64 编码的图片数据（`image` 字段以 `data:image/...` 开头）
/// - 图片文件路径（`image` 字段为文件路径，适配器内部通过 pawbun-files 加载）
pub struct OpenAiVisionTool {
    api_key: String,
    api_base: String,  // 默认 "https://api.openai.com/v1"
    model: String,     // 默认 "gpt-4o"
    client: reqwest::Client,
}

impl OpenAiVisionTool {
    pub fn new(api_key: impl Into<String>) -> Self;
    pub fn with_base(mut self, base: impl Into<String>) -> Self;
    pub fn with_model(mut self, model: impl Into<String>) -> Self;
}

#[async_trait::async_trait]
impl AsyncTool for OpenAiVisionTool {
    async fn execute_async(&self, input: &str) -> Result<ToolResult, ToolError> {
        // 1. 解析 input JSON：image (string), prompt (string, optional)
        // 2. 判断 image 是 data URI 还是路径
        //    - 路径：通过 pawbun-files 的 DefaultFileLoader 加载 → base64
        //    - data URI：直接使用
        // 3. 构造 OpenAI chat.completions 请求体
        //    messages: [{ role: "user", content: [
        //      { type: "text", text: prompt },
        //      { type: "image_url", image_url: { url: "data:image/png;base64,..." } }
        //    ]}]
        // 4. POST /chat/completions
        // 5. 解析响应，提取 choices[0].message.content
        // 6. 返回 ToolResult
    }
}
```

**与 `pawbun-files` 的集成**：
- 通过 `pawbun_files::{File, DefaultFileLoader, FileLoader}` 加载图片，复用已有的沙箱路径校验和图片约束。
- 图片过大时，利用 `pawbun_files::constraints::downgrade_image()` 自动压缩至目标尺寸。

### 3.4 EmbeddingTool → OpenAI Embedding 适配器

```rust
use pawbun_toolkit::{Tool, ToolResult, ToolError, AsyncTool, EmbeddingTool};

/// OpenAI text-embedding-3 适配器。
pub struct OpenAiEmbeddingTool {
    api_key: String,
    api_base: String,
    model: String,  // 默认 "text-embedding-3-small"
    client: reqwest::Client,
}

#[async_trait::async_trait]
impl AsyncTool for OpenAiEmbeddingTool {
    async fn execute_async(&self, input: &str) -> Result<ToolResult, ToolError> {
        // 1. 解析 input JSON：text (string | string[]), model (string, optional)
        // 2. POST /embeddings
        // 3. 解析响应，提取 embedding 数组
        // 4. 返回 JSON 序列化后的 ToolResult
    }
}
```

### 3.5 适配器的组织方式

**方案 A：独立示例目录（推荐）**

```
Pawbun/
├── crates/
│   └── pawbun-toolkit/
│       └── examples/
│           ├── docker_code_executor.rs    # DockerCodeExecutor 完整实现
│           ├── openai_vision.rs           # OpenAiVisionTool 完整实现
│           └── openai_embedding.rs        # OpenAiEmbeddingTool 完整实现
```

优点：零运行时依赖负担，纯文档性质，用户按需复制。

**方案 B：可选 feature（备选）**

若社区有强烈需求，可在 `pawbun-toolkit` 中新增 feature：
- `sandbox-docker` → `DockerCodeExecutor`
- `vision-openai` → `OpenAiVisionTool`
- `embedding-openai` → `OpenAiEmbeddingTool`

0.2.0 先采用方案 A，后续根据反馈决定是否升级至方案 B。

---

## 4. 错误上下文增强

### 4.1 当前问题

0.1.0 的 `ToolError` 缺乏**链式追溯能力**——`ExecutionFailed(String)` 等变体为平面字符串，丢失根因：

```rust
// 当前：错误信息平面化，丢失根因
ToolError::ExecutionFailed("failed to load file".into())

// 理想：可逐层追溯
ToolError::ExecutionFailed {
    message: "file_read tool failed".into(),
    source: Some(Box::new(io_err)),
}
```

`McpServerError` 当前已使用 `#[from]`（如 `Transport(#[from] TransportError)`），`thiserror` 自动实现了 `Error::source()`，**无需改动**。

### 4.2 设计目标

| 目标 | 说明 |
|---|---|
| `#[source]` 链式错误 | `ToolError` 的 `InvalidInput`、`ExecutionFailed`、`Serialization` 变体增加 `source` 字段，通过 `std::error::Error::source()` 访问。 |
| 向后兼容 | 保留 `ToolError::ExecutionFailed(String)` 等快捷构造函数；`McpServerError` 已有 `#[from]`，无需修改。 |
| Display 增强 | `thiserror` 的 `#[error("...")]` 格式需包含所有可用上下文。 |

### 4.3 具体改动

#### `ToolError`

```rust
#[derive(thiserror::Error, Debug, Clone)]
pub enum ToolError {
    #[error("invalid input: {message}")]
    InvalidInput {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("execution failed: {message}")]
    ExecutionFailed {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("tool not found: {0}")]
    NotFound(String),

    #[error("timeout after {0}ms")]
    Timeout(u64),

    #[error("serialization error: {message}")]
    Serialization {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

// 保留向后兼容的快捷构造函数
impl ToolError {
    pub fn invalid_input(msg: impl Into<String>) -> Self {
        Self::InvalidInput { message: msg.into(), source: None }
    }
    pub fn execution_failed(msg: impl Into<String>) -> Self {
        Self::ExecutionFailed { message: msg.into(), source: None }
    }
    pub fn with_source(self, source: impl std::error::Error + Send + Sync + 'static) -> Self {
        match self {
            Self::InvalidInput { message, .. } => Self::InvalidInput {
                message,
                source: Some(Box::new(source)),
            },
            Self::ExecutionFailed { message, .. } => Self::ExecutionFailed {
                message,
                source: Some(Box::new(source)),
            },
            Self::Serialization { message, .. } => Self::Serialization {
                message,
                source: Some(Box::new(source)),
            },
            other => other,
        }
    }
}
```

#### `LoadError`（pawbun-files）

0.2.0 **不改动**。当前变体均为自身校验结果（`Io`、`Network`、`PathTraversal`、`TypeMismatch`、`SizeExceeded`、`UnsupportedFormat`），不包装外部错误。

#### `McpServerError`

0.2.0 **不改动**。当前已通过 `#[from]` 实现自动链式错误：

```rust
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

**兼容性策略**：
- `ToolError::ExecutionFailed(String)` → 改为 `ToolError::ExecutionFailed { message, source }`，但保留 `ExecutionFailed(String)` 作为快捷构造函数。
- `McpServerError` 已有 `#[from]` 实现，自动支持 `Error::source()`，0.2.0 不改动。
- `LoadError`（`pawbun-files`）当前变体均为自身校验结果，0.2.0 不改动。

0.2.0 采用**直接修改**：因尚未发布 1.0，API 可在 minor 版本中演进。

---

## 5. MCP 服务器配置化

### 5.1 当前限制

0.1.0 的 `McpServerBuilder` 仅支持：
- `register_toolkit(toolkit)`
- `register_file_loader(loader)`
- `register_tool(tool)`
- `capabilities(Value)`

以下参数被**硬编码**：
- 协议版本：`"2024-11-05"`
- ServerInfo 的 name/version：由 builder 构造函数传入，但 capabilities 结构固定为 `{"tools": {}}`

### 5.2 新增 Builder 方法

```rust
impl McpServerBuilder {
    /// 覆盖默认 MCP 协议版本。
    /// 默认："2024-11-05"
    pub fn protocol_version(mut self, version: impl Into<String>) -> Self;

    /// 精细控制 capabilities（替代现有的 `capabilities(Value)`）。
    /// 提供类型安全的方法，避免用户手写 JSON。
    pub fn with_tools_capability(mut self) -> Self;
    pub fn with_logging_capability(mut self, level: LogLevel) -> Self;
    pub fn with_prompts_capability(mut self) -> Self;
    pub fn with_resources_capability(mut self) -> Self;

    /// SSE 传输配置。
    /// 仅在 `http` feature 下可用。
    #[cfg(feature = "http")]
    pub fn cors_origins(mut self, origins: Vec<String>) -> Self;

    /// 单次请求超时（从收到 JSON-RPC 请求到返回响应的总时间）。
    /// 默认：30s。
    /// 作用域：影响所有传输方式。在 `RequestHandler::handle` 层面统一应用，
    /// 防止单个请求/工具调用导致 server 无响应。
    pub fn request_timeout(mut self, ms: u64) -> Self;

    /// 单工具调用超时（`tools/call` 内部调用 `ToolKit::execute` 的时间）。
    /// 默认：继承 ToolKit 的 default_timeout_ms。
    /// 优先级：tool_timeout（MCP 层面）覆盖 ToolKit 默认值；若两者均未设置则无超时。
    /// 实现：`RequestHandler::handle_call_tool` 中用 `tokio::time::timeout`（SSE）或线程超时（stdio）包装。
    pub fn tool_timeout(mut self, ms: u64) -> Self;
}
```

### 5.3 Capabilities 类型安全

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

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapability {
    pub list_changed: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingCapability {
    pub level: LogLevel,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}
```

`McpServerBuilder` 内部从 `Value` 改为 `ServerCapabilities`，但保留 `capabilities(Value)` 作为兜底（向后兼容）。

### 5.4 CORS 配置

SSE 传输在 axum 层增加 CORS 中间件：

```rust
#[cfg(feature = "http")]
pub struct SseServerConfig {
    pub bind_addr: String,
    pub cors_origins: Vec<String>,
    pub cors_methods: Vec<String>,
    pub cors_headers: Vec<String>,
    pub heartbeat_interval_ms: u64,  // 默认 15000
    pub heartbeat_text: String,       // 默认 "ping"
}
```

Builder 链式调用：
```rust
let server = McpServer::builder("pawbun", "0.2.0")
    .protocol_version("2024-11-05")
    .with_tools_capability()
    .cors_origins(vec!["https://app.pandaria.dev".into()])
    .request_timeout(60_000)
    .register_toolkit(toolkit)
    .build();
```

---

## 6. SSE 传输稳定性

### 6.1 当前问题

0.1.0 的 `SseServerTransport` 实现了基本的 SSE 握手和 POST 路由，已有 `KeepAlive` 级别的心跳（15s 间隔发送空行 `ping`），但缺乏：
- **客户端重连参数不可配置**（`SseTransport` 已有指数退避重连，但 `max_retries`/`backoff_ms` 未暴露给调用方）
- **服务端心跳可配置性**（间隔、内容均硬编码）
- **并发连接管理**（无上限、无 TTL 清理）

### 6.2 重连机制增强

**服务端**：
- 每个 SSE 连接分配唯一 `session_id`（UUID v4）。
- 在 `endpoint` 事件中返回 `session_id`。
- 客户端 POST `/message?session_id=xxx` 必须携带匹配的 `session_id`。
- 连接断开后，`session_id` 在 TTL（如 30s）内保留，允许客户端通过 `Last-Event-ID` 恢复。
- 增加最大连接数限制，超限拒绝新连接。

**客户端侧（McpAdapter）**：
当前 `SseTransport` 内部已实现指数退避重连（1s → max 60s），但参数未暴露。0.2.0 增加配置接口：

```rust
impl SseTransport {
    pub fn with_retry(mut self, max_retries: u32, initial_backoff_ms: u64) -> Self;
}
```
- 断线后按指数退避重试，最多 `max_retries` 次。
- 重连成功后，发送 `Last-Event-ID` 头部以恢复事件流。

### 6.3 心跳机制增强

当前已有 `KeepAlive::new().interval(Duration::from_secs(15)).text("ping")`，0.2.0 增强为可配置：

```rust
// 伪代码——实现时参照 axum 实际 API
async fn sse_stream(
    state: Arc<AppState>,
    session_id: Uuid,
    config: SseServerConfig,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let events = // ... 正常事件流

    Sse::new(events).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(Duration::from_millis(config.heartbeat_interval_ms))
            .text(&config.heartbeat_text),
    )
}
```

### 6.4 连接管理

```rust
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct SseSessionManager {
    sessions: Arc<RwLock<HashMap<Uuid, SseSession>>>,
    max_connections: usize,
}

struct SseSession {
    created_at: Instant,
    last_activity: Instant,
    sender: mpsc::Sender<JsonRpcResponse>,
}

impl SseSessionManager {
    /// 新建会话，超限则返回错误
    pub async fn create(&self) -> Result<Uuid, TransportError>;
    /// 清理超时会话（由定时任务调用）
    pub async fn gc_expired(&self, ttl: Duration);
}
```

**依赖策略**：使用 `Arc<RwLock<HashMap>>` 而非 `dashmap`，0.2.0 不引入新依赖。后续有性能需求时再评估 `dashmap`。

---

## 7. 实施计划

### Phase 1：错误增强（P1）
- 修改 `ToolError`：将 `InvalidInput`、`ExecutionFailed`、`Serialization` 从元组变体改为结构体变体（增加 `source` 字段）
- 保留快捷构造函数（`invalid_input(msg)`、`execution_failed(msg)`）以维持向后兼容
- 更新所有现有调用点，在适当位置填充 `source`
- `McpServerError` 和 `LoadError` 不改动
- **不新增功能**，纯重构

### Phase 2：MCP 服务器配置化（P1）
- 实现 `ServerCapabilities` 类型安全结构（含 `#[serde(rename_all = "camelCase")]`）
- 扩展 `McpServerBuilder`（protocol_version / capabilities / CORS / timeout）
- 修改 `RequestHandler::new`，接收 `protocol_version: String` 参数
- 修改 `handle_initialize`，与传入的 `protocol_version` 比较（替代硬编码）
- SSE CORS 配置（依赖：`tower-http = { version = "0.5", features = ["cors"], optional = true }`，纳入 `http` feature）
- `RequestHandler::handle_call_tool` 增加超时包装（`tokio::time::timeout` / 线程超时）

### Phase 3：SSE 稳定性（P1）
- 会话 ID 管理
- 心跳机制
- 客户端重连

### Phase 4：适配器示例（P0）
- `DockerCodeExecutor` 示例
- `OpenAiVisionTool` 示例
- `OpenAiEmbeddingTool` 示例

### Phase 5：验收
- `cargo test --workspace --all-features` 全绿
- `cargo clippy --workspace --all-features -- -D warnings` 零警告
- 所有新增示例可独立 `cargo run --example xxx`

---

## 8. 验收标准

| 检查项 | 标准 |
|---|---|
| 编译 | `cargo check --workspace --all-features` 零错误 |
| Clippy | `cargo clippy --workspace --all-features -- -D warnings` 零警告 |
| 测试 | `cargo test --workspace --all-features` 全绿（含新增测试） |
| 文档测试 | `cargo test --workspace --all-features --doc` 全绿 |
| 示例运行 | 每个适配器示例可 `cargo run --example xxx` 直接运行 |
| 向后兼容 | 0.1.0 的公开 API 在 0.2.0 中仍可用（`ToolError::ExecutionFailed(String)` 等快捷构造函数保留） |

---

## 9. 相关文档

- [pawbun-toolkit-spec.md](pawbun-toolkit-spec.md) — 工具层核心设计
- [pawbun-mcp-server-spec.md](pawbun-mcp-server-spec.md) — MCP 服务器设计
- [pawbun-files-spec.md](pawbun-files-spec.md) — 文件处理层设计
- [../VERSIONS.md](../VERSIONS.md) — 版本记录与路线图
