# pawbun-toolkit Specification

> Version: 0.1.0-draft  
> Status: Design  
> Date: 2026-05-20

---

## 1. 目标与范围

`pawbun-toolkit` 为 Pandaria 生态的 Agent 提供**可发现、可调用、可组合**的工具抽象层。设计参考 [CrewAI Tools](https://github.com/crewAIInc/crewAI/tree/main/lib/crewai-tools)，但以 Rust 的类型安全和零成本抽象为核心优势。

### 1.1 核心目标
- 提供统一的 `Tool` trait，屏蔽具体实现差异。
- 支持同步与异步执行模型。
- 支持结构化输入（JSON Schema）与输出，便于 Agent 理解和调用。
- 提供内置常用工具集，开箱即用。
- 预留 MCP (Model Context Protocol) 适配接口，兼容外部工具生态。

### 1.2 非目标
- 不实现具体 LLM 客户端（由 Pandaria 其他模块负责）。
- 不强制要求任何运行时（如 tokio），异步支持通过标准 `Future` 实现。

---

## 2. 架构概述

```
┌──────────────────────────────────────────────────────────────┐
│  项目外：Pandaria Agent / Workflow / 用户编排代码             │
│  - LLM 调用与决策                                             │
│  - 多步骤工作流编排 (ToolChain)                               │
│  - 业务逻辑编排                                               │
└───────────────────────┬──────────────────────────────────────┘
                        │ trait 边界：ToolRegistry + ToolExecutor
┌───────────────────────▼──────────────────────────────────────┐
│  pawbun-toolkit（本项目）                                     │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  ToolKit（默认实现）                                     │ │
│  │  - 注册表 (HashMap<String, Box<dyn Tool>>)              │ │
│  │  - 单工具调用分发                                        │ │
│  │  - 元数据索引 (descriptions)                             │ │
│  └─────────────────────────────────────────────────────────┘ │
│                          │                                   │
│          ┌───────────────┼───────────────┐                   │
│          ▼               ▼               ▼                   │
│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐         │
│  │  Built-in    │ │  Custom      │ │  MCP Adapter │         │
│  │  Tools       │ │  Tools       │ │  (External)  │         │
│  └──────────────┘ └──────────────┘ └──────────────┘         │
└──────────────────────────────────────────────────────────────┘
```

---

## 3. 核心概念与接口

### 3.1 Tool Trait（基础版）

当前已实现的基础接口：

```rust
pub trait Tool: std::fmt::Debug + Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, input: &str) -> Result<String, Box<dyn std::error::Error>>;
}
```

### 3.2 Tool Trait（扩展版，Spec 目标）

采用 **双 trait 设计**：`Tool`（同步）+ `AsyncTool`（异步），避免强制同步实现者承担 `Pin<Box<dyn Future>>` 的堆分配开销。

```rust
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;

/// 工具的输入参数描述（JSON Schema 子集）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub schema: Value, // JSON Schema fragment
}

/// 统一工具执行结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub content: String,
    pub metadata: Option<Value>,
    /// 执行耗时（毫秒），由调用方或拦截层填充。
    pub elapsed_ms: Option<u64>,
}

/// 同步工具核心 trait。
///
/// 所有内置工具与用户自定义工具必须实现此 trait。
pub trait Tool: std::fmt::Debug + Send + Sync {
    /// 工具唯一标识。
    fn name(&self) -> &str;

    /// 工具功能描述（供 Agent 理解）。
    fn description(&self) -> &str;

    /// 输入参数元数据。
    ///
    /// 返回 `Cow` 以允许编译期常量切片或运行时动态生成，避免不必要的堆分配。
    fn parameters(&self) -> Cow<'static, [ToolParameter]>;

    /// 同步执行入口。
    ///
    /// `input` 为 Agent 生成的原始字符串（通常是 JSON）。工具内部应自行解析为结构化参数。
    fn execute(&self, input: &str) -> Result<ToolResult, ToolError>;

    /// 以 `serde_json::Value` 形式执行，默认将 Value 序列化为字符串后调用 `execute`。
    fn execute_value(&self, input: Value) -> Result<ToolResult, ToolError> {
        let raw = serde_json::to_string(&input)
            .map_err(|e| ToolError::Serialization(e.to_string()))?;
        self.execute(&raw)
    }

    /// 将自身转换为异步工具引用，用于 `ToolKit` 的调度层。
    ///
    /// 若未实现 `AsyncTool`，返回 `None`。
    fn as_async(&self) -> Option<&dyn AsyncTool> {
        None
    }
}

/// 异步工具扩展 trait。
///
/// 需要异步 IO 的工具（如网络请求、MCP 调用）应额外实现此 trait。
/// 未实现此 trait 的工具可通过 `ToolKit` 的包装器在异步上下文中以阻塞方式运行。
///
/// **MSRV**：需要 Rust 1.75+（原生 `async fn` in trait）。
/// 若需 `dyn AsyncTool`（trait object），当前 Rust 版本需手动将返回类型 boxing 为
/// `Pin<Box<dyn Future<Output = ...> + Send>>`。
pub trait AsyncTool: Tool {
    async fn execute_async(&self, input: &str) -> Result<ToolResult, ToolError>;

    async fn execute_value_async(&self, input: Value) -> Result<ToolResult, ToolError> {
        let raw = serde_json::to_string(&input)
            .map_err(|e| ToolError::Serialization(e.to_string()))?;
        self.execute_async(&raw).await
    }
}
```

#### 设计决策：为什么不用单一 `async` trait？

| 方案 | 优点 | 缺点 | 结论 |
|---|---|---|---|
| **单一 async trait**（`async-trait`） | API 统一，调用方无感知 | 同步工具也被迫装箱为 `Pin<Box<dyn Future>>`，每次调用堆分配 | ❌ 放弃 |
| **trait 内 `execute_async` 默认实现**（原草案） | 无外部依赖 | 默认实现 `Box::pin(...)` 仍有堆分配；object-safe 限制多 | ❌ 放弃 |
| **双 trait：`Tool` + `AsyncTool`** | 同步工具零开销；异步工具显式声明；`ToolKit` 可统一调度 | 需要实现两个 trait（但可通过宏简化） | ✅ 采纳 |

`ToolExecutor::execute_async` 的实现逻辑：
1. 若目标工具实现了 `AsyncTool`，直接调用 `execute_async`。
2. 否则，通过 `BlockingExecutor` trait 将同步 `execute` 投递到阻塞线程池：

```rust
/// 可插拔的阻塞执行策略，由调用方根据所用运行时提供实现。
pub trait BlockingExecutor: Send + Sync {
    fn spawn_blocking<F, R>(&self, f: F) -> Pin<Box<dyn Future<Output = R> + Send>>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static;
}

// tokio 用户提供的适配器示例：
pub struct TokioExecutor;
impl BlockingExecutor for TokioExecutor {
    fn spawn_blocking<F, R>(&self, f: F) -> Pin<Box<dyn Future<Output = R> + Send>>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        Box::pin(tokio::task::spawn_blocking(f))
    }
}
```

---

### 3.3 与外部编排器的 Trait 边界

pawbun-toolkit **不提供** Agent 决策或多步骤编排逻辑。这些由 Pandaria 生态其他 crate 或用户代码实现。

框架通过两个 trait 暴露能力，外部编排器只依赖 trait，不依赖 `ToolKit` 具体类型：

```rust
/// 工具发现与注册能力。
pub trait ToolRegistry: Send + Sync {
    /// 按名称获取工具。
    fn get(&self, name: &str) -> Option<&dyn Tool>;

    /// 列出所有已注册工具。
    fn list(&self) -> Vec<&dyn Tool>;

    /// 生成给 LLM 的 function-calling 风格描述文本。
    fn descriptions(&self) -> String;
}

/// 单工具同步执行能力。
pub trait ToolExecutor: Send + Sync {
    /// 同步执行指定工具。
    fn execute(&self, name: &str, input: &str) -> Result<ToolResult, ToolError>;
}

/// 单工具异步执行能力（Alpha 阶段引入）。
///
/// **MSRV**：需要 Rust 1.75+（原生 `async fn` in trait）。
pub trait AsyncToolExecutor: ToolExecutor {
    /// 异步执行指定工具。
    ///
    /// 若目标工具实现了 `AsyncTool`，直接调用 `execute_async`。
    /// 否则，通过 `BlockingExecutor` 将同步 `execute` 投递到阻塞线程池。
    async fn execute_async(
        &self,
        name: &str,
        input: &str,
        executor: &dyn BlockingExecutor,
    ) -> Result<ToolResult, ToolError>;
}
```

`ToolKit` 是上述两个 trait 的默认实现：

```rust
pub struct ToolKit {
    tools: HashMap<String, Box<dyn Tool>>,
    default_timeout_ms: Option<u64>,
}

impl ToolKit {
    pub fn new() -> Self;
    pub fn with_timeout(ms: u64) -> Self;
    pub fn register(&mut self, tool: Box<dyn Tool>);
}

impl ToolRegistry for ToolKit { /* ... */ }
impl ToolExecutor for ToolKit { /* ... */ }

// Alpha 阶段：
impl AsyncToolExecutor for ToolKit { /* ... */ }
```

外部编排器（Agent / Workflow）的使用方式：

```rust
use pawbun_toolkit::{ToolRegistry, ToolExecutor};

// Agent 只依赖 trait，不关心具体是 ToolKit 还是其他实现
pub struct Agent<R: ToolRegistry, E: ToolExecutor> {
    registry: R,
    executor: E,
}

impl<R: ToolRegistry, E: ToolExecutor> Agent<R, E> {
    pub fn run(&self, task: &str) -> Result<String, ToolError> {
        // 1. 获取工具描述，构造 LLM prompt
        let desc = self.registry.descriptions();
        // 2. 调用 LLM，获取 function call 指令
        // 3. 通过 executor 执行工具
        let result = self.executor.execute("file_read", "...")?;
        Ok(result.content)
    }
}

/// 异步 Agent（Alpha 阶段）。
pub struct AsyncAgent<R: ToolRegistry, E: AsyncToolExecutor> {
    registry: R,
    executor: E,
}
```

#### 超时策略

超时控制由 **ToolKit 调用层** 负责，而非单个工具内部实现，原因：
- 避免每个工具重复实现超时逻辑；
- 超时策略属于调用方契约（如 Agent 可配置全局或单调用超时）。

实现方式：
- **同步上下文**：使用 `std::thread::spawn` + `thread::park_timeout` 或跨通道接收超时信号。
- **异步上下文**：使用运行时提供的 `timeout`（如 `tokio::time::timeout`）。超时到达后向工具发送取消信号（若工具支持协作式取消）。

```rust
impl ToolKit {
    /// 单调用超时执行（同步）。
    pub fn execute_with_timeout(
        &self,
        name: &str,
        input: &str,
        timeout_ms: u64,
    ) -> Result<ToolResult, ToolError> {
        // 内部使用线程或通道实现超时中断
    }
}
```

### 3.4 ToolError

```rust
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ToolError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    #[error("tool not found: {0}")]
    NotFound(String),

    #[error("timeout after {0}ms")]
    Timeout(u64),

    #[error("serialization error: {0}")]
    Serialization(String),

}
```

---

## 4. 内置工具分类

| 类别 | 工具名 | 功能 | 优先级 |
|---|---|---|---|
| **文件** | `FileReadTool` | 读取文件内容 | P0 |
| | `FileWriteTool` | 写入文件内容 | P0 |
| | `DirectoryListTool` | 列出目录内容 | P1 |
| **网络** | `WebSearchTool` | 搜索引擎查询 | P0 |
| | `WebFetchTool` | 抓取网页内容 | P1 |
| **代码** | `CodeExecuteTool` | 在沙箱中执行代码片段（需外部沙箱运行时） | P1 |
| **数据** | `JsonQueryTool` | 使用 JSONPath/JMESPath 查询 | P2 |
| | `CsvQueryTool` | CSV 过滤与聚合 | P2 |
| **AI** | `VisionTool` | 图片理解与描述 | P1 |
| | `EmbeddingTool` | 文本向量化 | P2 |

### 4.1 示例：FileReadTool 接口

`FileReadTool` 基于 `pawbun-files` 的 `DefaultFileLoader` 实现，支持多模态内容读取（文本、图片、PDF 等），返回序列化后的 `MediaContent` JSON。

```rust
use pawbun_files::{File, DefaultFileLoader, FileLoader, MediaContent};
use std::path::PathBuf;

#[derive(Debug)]
pub struct FileReadTool {
    loader: DefaultFileLoader,
}

impl FileReadTool {
    pub fn new() -> Self {
        Self { loader: DefaultFileLoader::new() }
    }

    pub fn with_base_dir<P: Into<PathBuf>>(mut self, base_dir: P) -> Self {
        self.loader = DefaultFileLoader::with_base_dir(base_dir);
        self
    }
}

impl Tool for FileReadTool {
    fn name(&self) -> &str { "file_read" }

    fn description(&self) -> &str {
        "Read the contents of a file and return structured MediaContent. \
         Supports text, images, PDFs, audio, and video."
    }

    fn parameters(&self) -> Cow<'static, [ToolParameter]> {
        Cow::Owned(vec![
            ToolParameter {
                name: "path".into(),
                description: "Relative or absolute file path".into(),
                required: true,
                schema: serde_json::json!({"type": "string"}),
            }
        ])
    }

    fn execute(&self, input: &str) -> Result<ToolResult, ToolError> {
        // input 为 JSON: {"path": "./report.pdf"}
        let path: String = serde_json::from_str(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        let file = File::from_path(&path);

        // 加载（路径安全由 DefaultFileLoader 统一处理）
        let loaded = self.loader.load(&file)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // 工具间传递：序列化 MediaContent（bytes 字段自动 Base64）
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
```

---

## 5. 扩展机制

### 5.1 自定义工具（实现 Trait）

最灵活的方式，直接实现 `Tool` trait。

### 5.2 函数式工具宏（未来）

参考 CrewAI 的 `@tool` 装饰器，未来可提供过程宏：

```rust
// 未来语法（非当前实现目标）
#[pawbun_tool(name = "calculator", description = "Evaluate math expressions")]
fn calculator(expr: String) -> Result<String, ToolError> {
    // ...
}
```

### 5.3 动态工具加载

```rust
pub trait ToolLoader: Send + Sync {
    fn load(&self, config: &ToolConfig) -> Result<Box<dyn Tool>, ToolError>;
}

pub struct ToolConfig {
    pub name: String,
    pub source: ToolSource,
    pub options: HashMap<String, String>,
}

pub enum ToolSource {
    Builtin,
    PluginPath(std::path::PathBuf),
    McpConfig(mcp::TransportConfig),
}
```

---

## 6. MCP 集成设计

Model Context Protocol (MCP) 让 Agent 能够无缝调用外部服务器提供的工具。

### 6.1 适配层架构

```
┌──────────────────┐
│   MCP Server     │  (外部进程 / SSE 服务)
│  (stdio / sse)   │
└────────┬─────────┘
         │ MCP Protocol (JSON-RPC)
┌────────▼─────────┐
│ MCPClientAdapter │  ← 封装通信细节
└────────┬─────────┘
         │ 转换为 Tool trait
┌────────▼─────────┐
│   dyn Tool       │  ← 融入 ToolKit
└──────────────────┘
```

### 6.2 接口草案

```rust
pub mod mcp {
    use super::*;
    use std::path::PathBuf;

    pub enum TransportConfig {
        Stdio {
            command: String,
            args: Vec<String>,
            /// 允许访问的目录白名单；`None` 表示仅允许临时目录。
            allowed_paths: Option<Vec<PathBuf>>,
        },
        Sse {
            url: String,
            /// 是否校验 TLS 证书；生产环境必须为 `true`。
            verify_tls: bool,
        },
    }

    pub struct McpAdapter {
        config: TransportConfig,
    }

    impl McpAdapter {
        pub async fn connect(&self) -> Result<McpSession, ToolError>;
        pub async fn list_tools(&self, session: &McpSession) -> Result<Vec<Box<dyn Tool>>, ToolError>;
    }

    /// MCP 会话句柄。
    ///
    /// **注意**：不提供 `Drop` 实现，因为异步清理无法在安全同步的 `drop` 中完成。
    /// 调用者必须使用 `session.close().await`，否则连接可能泄漏。
    #[must_use = "MCP sessions should be explicitly closed with `.close().await`"]
    pub struct McpSession {
        // 内部持有连接状态
    }

    impl McpSession {
        /// 优雅关闭连接并释放资源。
        pub async fn close(self) -> Result<(), ToolError>;
    }
}
```

### 6.3 安全约束
- STDIO MCP 服务器将在本地执行代码，必须通过配置白名单控制。
- SSE 连接需要验证服务器 TLS 证书。
- 对 MCP 返回的文本输出进行转义，防止 prompt injection。

---

## 7. 依赖策略

| 依赖 | 用途 | 是否必须 |
|---|---|---|
| `serde` + `serde_json` | 结构化输入输出 | 是 |
| `thiserror` | 错误定义 | 是 |
| — | 异步 trait 基于 Rust 1.75+ 原生 `async fn` | 否（Alpha 引入，需 MSRV 1.75） |
| `schemars` | 自动生成 JSON Schema | 否（P1）|
| `tracing` | 调用链追踪与日志 | 否（P1）|
| `tokio` | 异步运行时（dev-dep） | 仅测试 |
| `reqwest` | HTTP 工具实现 | 内置工具可选依赖 |

---

## 7.1 可观测性

生产环境需追踪工具调用链路。采用 **可选依赖** `tracing` 实现：

- 每个 `Tool::execute` 调用自动生成一个 `span`，包含 `tool.name` 和调用参数摘要（脱敏后）。
- `ToolResult::elapsed_ms` 由 `ToolKit` 拦截层自动填充，无需工具自身实现。
- 外部编排器（如 Pandaria Workflow）的每一步可生成独立子 span，便于定位失败步骤。

```rust
use tracing::{info, instrument};

impl ToolKit {
    #[instrument(skip(self, input), fields(tool = name))]
    pub fn execute(&self, name: &str, input: &str) -> Result<ToolResult, ToolError> {
        // ... 自动记录开始/结束/错误
    }
}
```

若用户未启用 `tracing` feature，所有观测逻辑通过条件编译 (`#[cfg(feature = "tracing")]`) 消除，零运行时开销。

---

## 8. 路线图

| 阶段 | 内容 | 预计版本 |
|---|---|---|
| **MVP** | 基础 `Tool` / `ToolKit` + `ToolResult` / `ToolError` + `FileReadTool` / `FileWriteTool` | 0.1.0 |
| **Alpha** | `AsyncTool` + 超时控制 + `WebSearchTool` / `WebFetchTool` | 0.2.0 |
| **Beta** | JSON Schema 自动生成 + 过程宏支持 | 0.3.0 |
| **Stable** | MCP 适配层 + 完整内置工具集 | 0.5.0 |
| **1.0** | API 冻结 + tracing 集成 + 性能优化 + 文档完善 | 1.0.0 |

---

## 9. 附录：与 CrewAI Tools 的映射

| CrewAI (Python) | pawbun-toolkit (Rust) | 说明 |
|---|---|---|
| `BaseTool` | `Tool` trait | 核心抽象 |
| `name: str` | `fn name(&self) -> &str` | 标识符 |
| `description: str` | `fn description(&self) -> &str` | 功能描述 |
| `_run(...)` | `fn execute(&self, ...)` | 同步执行 |
| `@tool` 装饰器 | 过程宏（未来） | 快速创建工具 |
| `MCPServerAdapter` | `mcp::McpAdapter` | MCP 适配 |
| 工具列表传给 Agent | `ToolKit::descriptions()` | 生成 tools prompt |

---

## 10. 附录：外部编排器实现参考

本节展示 Pandaria Agent（项目外）如何利用 `ToolRegistry` + `ToolExecutor` trait 实现多步骤编排，证明 trait 边界的实际价值。

### 10.1 简单的顺序执行编排器

```rust
use pawbun_toolkit::{ToolExecutor, ToolResult, ToolError};

/// 外部 crate 实现的顺序编排器，零依赖 ToolKit 具体类型。
pub struct SequentialExecutor<'a> {
    executor: &'a dyn ToolExecutor,
}

impl<'a> SequentialExecutor<'a> {
    pub fn new(executor: &'a dyn ToolExecutor) -> Self {
        Self { executor }
    }

    /// 顺序执行多个工具，FailFast 策略。
    pub fn run_sequence(
        &self,
        steps: &[(String, String)], // (tool_name, input)
    ) -> Result<Vec<ToolResult>, ToolError> {
        let mut results = Vec::new();
        for (name, input) in steps {
            let result = self.executor.execute(name, input)?;
            if !result.success {
                return Err(ToolError::ExecutionFailed(
                    format!("step {name} failed: {}", result.content)
                ));
            }
            results.push(result);
        }
        Ok(results)
    }
}
```

### 10.2 Agent 集成示例

```rust
use pawbun_toolkit::{ToolRegistry, ToolExecutor};

/// Pandaria Agent 的核心逻辑，只依赖 trait。
pub struct Agent<'a> {
    registry: &'a dyn ToolRegistry,
    executor: &'a dyn ToolExecutor,
}

impl<'a> Agent<'a> {
    pub fn new(registry: &'a dyn ToolRegistry, executor: &'a dyn ToolExecutor) -> Self {
        Self { registry, executor }
    }

    pub fn run_task(&self, task_description: &str) -> Result<String, ToolError> {
        // 1. 获取可用工具描述，构造 LLM prompt
        let tool_desc = self.registry.descriptions();
        let prompt = format!("{task_description}\n\nAvailable tools:\n{tool_desc}");

        // 2. 调用 LLM（由 Pandaria LLM crate 提供）
        // let llm_response = llm_client::complete(&prompt)?;

        // 3. 解析 LLM 返回的 function call（假设解析出 tool_name 和 input_json）
        // let (tool_name, input) = parse_function_call(&llm_response)?;

        // 4. 验证工具存在
        let _tool = self.registry.get(tool_name)
            .ok_or_else(|| ToolError::NotFound(tool_name.into()))?;

        // 5. 执行工具
        let result = self.executor.execute(tool_name, input)?;

        Ok(result.content)
    }
}
```

### 10.3 关键设计收益

| 场景 | 若 ToolKit 是具体类型 | 若使用 trait 边界 |
|---|---|---|
| 测试 Agent | 必须构造完整的 ToolKit | 可注入 mock registry/executor |
| 自定义注册表 | 无法替换（如从数据库加载工具） | 实现 `ToolRegistry` 即可 |
| 组合多个 ToolKit | 需要显式合并 HashMap | 实现代理 `ToolRegistry` 聚合多个来源 |
| 编译隔离 | Agent 依赖 ToolKit 全部实现 | Agent 只依赖 trait 定义，编译更快 |

---

## 相关文档

- [实施计划](../plans/pawbun-toolkit-implementation-plan.md) — 按路线图分 5 阶段的具体编码任务
