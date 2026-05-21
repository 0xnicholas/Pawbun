# pawbun-toolkit 实施计划

> 对应 Spec: [`docs/specs/pawbun-toolkit-spec.md`](../specs/pawbun-toolkit-spec.md)  
> Spec 版本: v0.1.0-draft  
> 计划日期: 2026-05-20

---

## 概述

按 Spec 路线图分 5 个阶段实施，每个阶段独立可发布。MVP 优先完成核心 trait 与文件工具，后续阶段逐步叠加异步、网络、MCP 能力。

---

## Phase 1: MVP (0.1.0) — 核心契约 + 文件工具

**目标**: 搭建可编译、可测试的工具抽象层，提供文件读写能力。

### 1.1 依赖升级

修改 `crates/pawbun-toolkit/Cargo.toml`：
- 引入 `serde = { version = "1", features = ["derive"] }`
- 引入 `serde_json = "1"`
- 引入 `thiserror = "1"`
- 引入 `pawbun-files = { path = "../pawbun-files" }`

### 1.2 核心类型与错误

新建/修改 `crates/pawbun-toolkit/src/lib.rs`：
- 定义 `ToolParameter` struct
- 定义 `ToolResult` struct
- 定义 `ToolError` enum（`InvalidInput`, `ExecutionFailed`, `NotFound`, `Timeout`, `Serialization`）

### 1.3 Tool trait 家族

- 定义 `Tool` trait：`name()`, `description()`, `parameters()`, `execute()`, `execute_value()`, `as_async()`
- `parameters()` 返回 `Cow<'static, [ToolParameter]>`
- 所有方法提供文档注释 + doc-test 示例

### 1.4 注册与执行边界

- 定义 `ToolRegistry` trait
- 定义 `ToolExecutor` trait（仅同步方法）
- 实现 `ToolKit` struct：`HashMap<String, Box<dyn Tool>>` + `default_timeout_ms`
- `impl ToolRegistry for ToolKit`
- `impl ToolExecutor for ToolKit`
- `ToolKit::register`, `get`, `list`, `descriptions`

### 1.5 内置工具：FileReadTool

新建 `crates/pawbun-toolkit/src/tools/file_read.rs`：
- `FileReadTool` struct（内部持有 `DefaultFileLoader`）
- `impl Tool for FileReadTool`
- 基于 `pawbun-files` 的 `DefaultFileLoader` 加载文件，支持多模态内容
- 路径安全委托给 `DefaultFileLoader`，无需重复实现
- 单测：正常读取、路径遍历拦截（通过 mock `FileLoader`）、多模态内容返回

### 1.6 内置工具：FileWriteTool

新建 `crates/pawbun-toolkit/src/tools/file_write.rs`：
- `FileWriteTool` struct
- `impl Tool for FileWriteTool`
- 同样的路径安全检查
- 单测：正常写入、目录自动创建、路径遍历拦截

### 1.7 模块组织

`src/lib.rs` 导出：
```rust
pub mod tools;
pub use crate::tools::{FileReadTool, FileWriteTool};
```

`src/tools/mod.rs` 组织内置工具模块。

### 1.8 文档与示例

- `lib.rs` crate 文档更新，包含 MVP 使用示例
- README 更新，增加使用示例

### 1.9 验收标准

```bash
cargo check --workspace   # 零错误
cargo test --workspace    # 所有单测通过
cargo doc --workspace     # 零警告，所有公共 API 有文档
```

---

## Phase 2: Alpha (0.2.0) — 异步 + 超时 + 网络工具

**目标**: 支持异步工具执行、超时控制、HTTP 工具。

### 2.1 依赖

- 异步 trait 基于 Rust 1.75+ 原生 `async fn`，无需 `async-trait` 依赖
- `Cargo.toml` 引入 `tokio = { version = "1", features = ["rt", "time"], optional = true }`（dev-dep 测试用）
- `Cargo.toml` 引入 `reqwest = { version = "0.12", optional = true }`

### 2.2 异步扩展

- 定义 `AsyncTool` trait（原生 `async fn`，继承 `Tool`）
- 定义 `AsyncToolExecutor` trait（原生 `async fn`，继承 `ToolExecutor`）
- `impl AsyncToolExecutor for ToolKit`
- `BlockingExecutor` trait + `TokioExecutor` 示例实现

### 2.3 超时控制

- `ToolKit::with_timeout(ms: u64)`
- `ToolKit::execute_with_timeout(name, input, timeout_ms)`
- `ToolExecutor` 默认 `execute` 自动应用 `default_timeout_ms`
- 超时实现：同步用线程/通道，异步用 `tokio::time::timeout`（如启用 tokio feature）

### 2.4 网络工具

新建 `src/tools/web_search.rs`：
- `WebSearchTool` struct（配置 API key、endpoint）
- `impl AsyncTool for WebSearchTool`
- 使用 `reqwest` 发送搜索请求

新建 `src/tools/web_fetch.rs`：
- `WebFetchTool` struct
- `impl AsyncTool for WebFetchTool`
- 抓取网页内容

### 2.5 验收标准

```bash
cargo test --workspace
cargo test --workspace --features tokio  # 异步测试
cargo test --workspace --features reqwest # 网络工具测试（mock）
```

---

## Phase 3: Beta (0.3.0) — JSON Schema + 过程宏

**目标**: 自动生成参数 JSON Schema，提供快速创建工具的宏。

### 3.1 JSON Schema

- `Cargo.toml` 引入 `schemars = { version = "0.8", optional = true }`
- `ToolParameter::schema` 从手动 `serde_json::Value` 改为 `schemars::schema::Schema`
- 为常用类型实现 `JsonSchema`，自动生成 schema

### 3.2 过程宏

新建 `crates/pawbun-toolkit-macros/`（workspace member）：
- `#[pawbun_tool(name = "...", description = "...")]` 宏
- 自动为函数生成 `Tool` impl
- 自动推导参数类型 → JSON Schema

### 3.3 验收标准

- 宏 crate 可独立编译
- 示例：`#[pawbun_tool]` 修饰函数后可直接 `ToolKit::register(Box::new(...))`

---

## Phase 4: Stable (0.5.0) — MCP + 完整内置工具

**目标**: MCP 适配层 + 剩余内置工具。

### 4.1 MCP 适配

新建 `src/mcp/`：
- `TransportConfig` enum（Stdio / Sse）
- `McpAdapter` struct
- `McpSession` struct + `close(self)` 方法
- `McpAdapter::connect()` + `list_tools()`
- MCP 返回的工具动态实现 `Tool` trait

### 4.2 剩余内置工具

- `DirectoryListTool`
- `JsonQueryTool`
- `CsvQueryTool`
- `VisionTool`
- `EmbeddingTool`
- `CodeExecuteTool`（标注需外部沙箱，仅提供接口）

### 4.3 验收标准

- MCP 单元测试（mock server）
- 所有内置工具有 doc-test

---

## Phase 5: 1.0 — API 冻结 + 可观测性 + 性能

**目标**: 生产就绪。

### 5.1 Tracing 集成

- `Cargo.toml` 引入 `tracing = { version = "0.1", optional = true }`
- `#[cfg(feature = "tracing")]` 条件编译
- `ToolKit::execute` 加 `#[instrument]`
- `ToolResult::elapsed_ms` 自动填充

### 5.2 API 审计

- 审查所有 `pub` API，移除不必要的暴露
- 确保向后兼容（semver 合规）

### 5.3 性能基准

- 同步工具调用 overhead < 1μs
- 注册表查找 O(1)

---

## 当前执行建议

**立即开始 Phase 1 (MVP)**。具体任务：

1. `crates/pawbun-toolkit/Cargo.toml` — 加依赖
2. `crates/pawbun-toolkit/src/error.rs` — `ToolError`
3. `crates/pawbun-toolkit/src/types.rs` — `ToolParameter`, `ToolResult`
4. `crates/pawbun-toolkit/src/tool.rs` — `Tool` trait
5. `crates/pawbun-toolkit/src/registry.rs` — `ToolRegistry`, `ToolExecutor`
6. `crates/pawbun-toolkit/src/toolkit.rs` — `ToolKit` 实现
7. `crates/pawbun-toolkit/src/tools/mod.rs` + `file_read.rs` + `file_write.rs`
8. `crates/pawbun-toolkit/src/lib.rs` — 模块导出 + 文档
