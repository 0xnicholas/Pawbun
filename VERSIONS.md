# Pawbun 版本记录

> 本文件记录 Pawbun workspace 各 crate 的版本状态与发布历史。
>
> 版本号遵循 [SemVer](https://semver.org/lang/zh-CN/)。

---

## 当前版本：0.2.0

**发布日期**：2026-05-26

**Workspace 版本**：`0.2.0`

| Crate | 版本 | 状态 | 说明 |
|---|---|---|---|
| `pawbun-toolkit` | 0.2.0 | ✅ 可用 | Agent 工具注册与执行核心（含适配器示例）|
| `pawbun-toolkit-macros` | 0.2.0 | ✅ 可用 | `#[pawbun_tool]` 过程宏 |
| `pawbun-files` | 0.2.0 | ✅ 可用 | 多模态文件加载与 Provider 格式化 |
| `pawbun-mcp-core` | 0.2.0 | ✅ 可用 | MCP 协议类型与传输抽象 |
| `pawbun-mcp-server` | 0.2.0 | ✅ 可用 | MCP 服务器（stdio / SSE，含配置化 Builder）|

---

## 功能概览

### pawbun-toolkit

- ✅ `Tool` / `AsyncTool` 双 trait 设计
- ✅ `ToolKit` 注册表（同步 + 异步执行 + 超时控制）
- ✅ `BlockingExecutor` / `TokioExecutor` 可插拔阻塞策略
- ✅ 内置工具（全部通过测试）：
  - `FileReadTool` — 读取文件，支持沙箱路径
  - `FileWriteTool` — 写入文件，自动创建目录
  - `DirectoryListTool` — 列出目录内容
  - `WebFetchTool` — HTTP 抓取（`http` feature）
  - `WebSearchTool` — 搜索调用（`http` feature）
  - `CsvQueryTool` — CSV 查询（`csv` feature）
  - `JsonQueryTool` — JSONPath 查询（`jsonpath` feature）
- ✅ 占位工具 + 适配器示例：
  - `CodeExecuteTool` — 占位接口 + `examples/docker_code_executor.rs`（Docker CLI 实现）
  - `EmbeddingTool` — 占位接口 + `examples/openai_embedding.rs`（OpenAI API 实现）
  - `VisionTool` — 占位接口 + `examples/openai_vision.rs`（OpenAI GPT-4o 实现）
- ✅ MCP 客户端（Stdio + SSE 传输，`DynamicTool` 代理远程工具）
- ✅ `ToolError` 链式错误（`#[source]` + `with_source()`）
- ✅ `SseTransport` 可配置重试（`new_with_retry`）

### pawbun-toolkit-macros

- ✅ `#[pawbun_tool(name = "...", description = "...")]` 属性宏
- ✅ 自动生成 `name()` / `description()` / `parameters()`
- ✅ 保留用户自定义方法（不覆盖已存在实现）

### pawbun-files

- ✅ `File` / `MediaType` / `MediaContent` 统一表示
- ✅ 三种来源：`Local` / `Url` / `Bytes`
- ✅ `DefaultFileLoader`：同步 + 异步 + 批量加载
- ✅ 四种 Provider 格式化：OpenAI / Anthropic / Gemini / Azure OpenAI
- ✅ 文件约束：大小限制、媒体类型白名单、溢出模式（Strict / Warn / Auto）
- ✅ 图片尺寸提取与自动降级（`image-meta` feature）
- ✅ 沙箱路径校验（`canonicalize` + 前缀检查）

### pawbun-mcp-core

- ✅ JSON-RPC 2.0 + MCP 消息类型（`JsonRpcRequest` / `JsonRpcResponse`）
- ✅ `ToolParameter` canonical 定义（被 toolkit 和 mcp-server 共享）
- ✅ `Transport` / `ServerTransport` trait + `TransportError`
- ✅ 双向 schema 转换：`schema_to_parameters` ↔ `parameters_to_schema`
- ✅ `schemars` feature（从类型自动生成 JSON Schema）

### pawbun-mcp-server

- ✅ `McpServer` + `McpServerBuilder`（支持 `protocol_version` / `capabilities` / `cors_origins` / `request_timeout` / `tool_timeout`）
- ✅ `RequestHandler`：initialize 状态机 + 方法路由
- ✅ 支持方法：`initialize` / `notifications/initialized` / `tools/list` / `tools/call`
- ✅ Stdio 传输（生产就绪）
- ✅ SSE 传输（`http` feature，基于 axum）
- ✅ `SseServerConfig`：可配置心跳、最大连接数、会话 TTL、CORS
- ✅ `FileLoader` → MCP Tool bridge（`file_read` / `file_list`）
- ✅ 工具去重：用户注册工具优先于 bridge 工具

---

## 质量指标

| 指标 | 数值 |
|---|---|
| 编译状态 | ✅ 零错误 |
| Clippy | ✅ 零警告 (`-D warnings`) |
| 单元测试 | ✅ 208 passed / 0 failed |
| 文档测试 | ✅ 21 passed / 0 failed |
| 代码行数 | ~17,800 行 Rust |
| 示例 | 3 个（docker_code_executor, openai_vision, openai_embedding）|
| 测试覆盖率 | 核心模块全覆盖（trait、工具、loader、provider、handler、transport） |

---

## 已知限制

1. **占位工具**：`CodeExecuteTool`、`EmbeddingTool`、`VisionTool` 仍为接口占位，但已提供官方适配器示例（`examples/` 目录），开发者可复制修改后使用。生产集成建议基于示例创建独立 crate。
2. **tracing 深度集成**：`tracing` feature 已存在，但未在所有关键路径加 `#[instrument]`。
3. **性能基准**：尚未建立正式的 benchmark 基线。
4. **无 CHANGELOG**：首次发布，尚无历史版本记录。

---

## 版本历史

### 0.2.0 — 生态集成

**范围**：ToolError 链式错误、MCP Server 配置化、SSE 稳定性、适配器示例

**主要提交**：
- `f997129` feat: implement Pawbun 0.2.0 ecosystem integration
  - `ToolError` 重构：`#[source]` 链式错误 + 向后兼容快捷构造函数
  - `ServerCapabilities` 类型安全结构 + `McpServerBuilder` 扩展
  - SSE 可配置心跳/连接限制/会话 TTL + 客户端重试参数暴露
  - 3 个适配器示例：DockerCodeExecutor、OpenAiVisionTool、OpenAiEmbeddingTool
  - 修复代理导致的 flaky 测试

### 0.1.0（初始版本）

**范围**：MVP → Alpha → Beta → Stable 的大部分功能

**主要提交**：
- `7c31b42` feat: initial implementation of Pawbun
- `dbdddc6` docs: update README with full project documentation
- `cab5fef` feat: add pawbun-mcp-core crate skeleton with ToolParameter
- `a21d1bb` feat(mcp-core): add protocol types with JsonRpcResponse constructors
- `b5387b0` feat(mcp-core): add transport traits, configs, and TransportError
- `4a6caa6` feat(mcp-core): add bidirectional schema_convert
- `2428da0` feat(mcp-server): add crate skeleton with error types and transport stub
- `ea458eb` feat(mcp-server): add RequestHandler with initialize state machine
- `cfb33c8` feat(mcp-server): add StdioServerTransport, McpServer launch, and FileLoader bridge
- `6858a23` feat(mcp-server): add SSE transport with session management and POST routing
- `b3c41e5` feat: migrate toolkit to reference pawbun-mcp-core, delete migrated mcp files
- `be9d1aa` fix: resolve remaining super:: protocol path references after migration
- `bd1e893` docs: add mcp-server spec and implementation plan

---

## 后续规划

---

### 0.2.0 — 生态集成 ✅ 已完成

**目标**：降低占位工具的集成门槛，让 Pawbun 与外部服务（沙箱、LLM、Embedding）顺畅对接。

| 工作项 | 优先级 | 状态 | 说明 |
|---|---|---|---|
| `CodeExecuteTool` 适配器示例 | P0 | ✅ | `examples/docker_code_executor.rs`：Docker CLI 沙箱，含资源限制与超时 |
| `VisionTool` 适配器示例 | P0 | ✅ | `examples/openai_vision.rs`：OpenAI GPT-4o 视觉分析 |
| `EmbeddingTool` 适配器示例 | P1 | ✅ | `examples/openai_embedding.rs`：OpenAI text-embedding-3 |
| 错误上下文增强 | P1 | ✅ | `ToolError` `#[source]` 链式错误 + `with_source()` + 向后兼容构造函数 |
| `pawbun-mcp-server` 配置化 | P1 | ✅ | `ServerCapabilities` 类型安全 + Builder 扩展（protocol_version / CORS / timeout）|
| SSE 传输稳定性 | P1 | ✅ | `SseServerConfig` 可配置心跳/最大连接数/会话 TTL；客户端 `new_with_retry` |

**验收标准**：
- ✅ `cargo test --workspace --all-features` 全绿（208 passed）
- ✅ `cargo clippy --workspace --all-features -- -D warnings` 零警告
- ✅ 3 个示例均独立编译通过

---

### 0.3.0 — 性能、API 审计与文档

**目标**：建立性能基线，精简公共 API，成为社区可信赖的依赖。

#### Phase 1: 基准测试（P0）

扩展现有 `crates/pawbun-toolkit/benches/toolkit.rs`：

| 基准项 | 说明 | 目标 |
|---|---|---|
| `toolkit_register` | 向 `ToolKit` 注册 1~1000 个工具的开销 | 单次注册 < 50μs |
| `toolkit_lookup` | `ToolKit::get` 查找 O(1) 实测 | < 100ns |
| `tool_execute_overhead` | `ToolKit::execute` 空工具调用 overhead | < 1μs |
| `tool_descriptions` | `ToolKit::descriptions()` 生成描述字符串 | < 1ms/100 tools |
| `json_schema_build` | `build_input_schema` 从 `ToolParameter` 构建 JSON Schema | < 10μs |
| `sse_parser` | SSE 事件解析吞吐 | > 1M events/sec |
| `file_load_local` | `DefaultFileLoader` 本地文件加载 | 与 std::fs::read 差距 < 2x |

新增 crate 级基准：
- `pawbun-files/benches/loader.rs` — 文件加载、Provider 格式化吞吐
- `pawbun-mcp-server/benches/handler.rs` — RequestHandler 初始化 + tools/list 响应

#### Phase 2: pub API 审计（P0）

逐 crate 审查所有 `pub`/`pub(crate)` 项：

| Crate | 审计重点 |
|---|---|
| `pawbun-toolkit` | `ToolKit` 字段是否应私有化；`json_utils` 是否应 `pub(crate)`；`mcp` 模块下的类型暴露粒度 |
| `pawbun-files` | `FileLoader` trait 方法是否需要精简；Provider 格式化器的内部辅助函数暴露 |
| `pawbun-mcp-core` | `JsonRpcId` / `JsonRpcError` 的构造函数是否完整；`schema_convert` 的边界情况 |
| `pawbun-mcp-server` | `RequestHandler` 是否应 `pub(crate)`；`tool_bridge` 模块暴露范围 |

潜在 breaking changes（0.3.0 是最后一次可调整的机会）：
- 将过度暴露的模块标记为 `#[doc(hidden)]` 或降级为 `pub(crate)`
- 统一命名风格（如 `ToolParameter::schema` 与 MCP 的 `input_schema`）
- 评估 `ToolKit::with_timeout` → `ToolKit::set_timeout` 等命名一致性

#### Phase 3: 文档增强（P1）

每个 crate 至少 2 个可运行示例：

| Crate | 示例 1 | 示例 2 |
|---|---|---|
| `pawbun-toolkit` | `examples/basic_toolkit.rs` — 注册工具 + 执行 | `examples/custom_tool.rs` — 手写 `Tool` trait 实现 |
| `pawbun-files` | `examples/load_image.rs` — 加载图片并格式化为 Provider | `examples/batch_load.rs` — 批量加载 + 约束配置 |
| `pawbun-mcp-server` | `examples/stdio_server.rs` — stdio MCP 服务器 | `examples/sse_server.rs` — SSE MCP 服务器 + CORS |
| `pawbun-mcp-core` | `examples/schema_convert.rs` — schema 双向转换 | `examples/custom_transport.rs` — 实现自定义 Transport |

文档目标：
- 所有 `pub` API 有完整 doc comment + `# Example`
- `README.md` 更新：新增 0.2.0 特性、快速开始示例
- 新增 `docs/cookbook.md`：常见场景指南（"如何添加自定义工具"、"如何配置 MCP 服务器"、"如何安全加载文件"）

#### Phase 4: 兼容性测试 & CI（P1）

Feature 组合矩阵测试（本地脚本 + CI）：

```bash
# 最小依赖集
cargo check --workspace --no-default-features
# 逐个 feature
cargo check --workspace --features http
cargo check --workspace --features tokio
cargo check --workspace --features csv
cargo check --workspace --features jsonpath
cargo check --workspace --features schemars
cargo check --workspace --features tracing
cargo check --workspace --features macros
# 全量
cargo check --workspace --all-features
```

验证 `default-features = false` 的消费者不会拉取不必要依赖。

#### Phase 5: 依赖精简（P2）

评估项：
- `image` crate（`pawbun-files` 的 `image-meta` feature）：是否可用更轻量的 `image-meta` 替代完整 `image`
- `schemars`：仅在 `macros`/`schemars` feature 需要时编译
- `reqwest`：评估是否可用更轻量的 `ureq` 替代同步场景
- `tokio`：确认 `pawbun-mcp-server` 的 `http` feature 下 `tokio` 的最小 feature set

**验收标准**：
- 基准测试报告纳入版本发布说明（`benches/README.md`）
- `cargo public-api`（如有）或人工审计确认无意外 breaking change
- docs.rs 上所有 crate 文档覆盖率 100%（`#![deny(missing_docs)]`）
- Feature 组合矩阵全部编译通过

---

### 0.4.0 — 扩展能力

**目标**：覆盖更多使用场景，提升与其他生态的互操作性。

| 工作项 | 优先级 | 说明 |
|---|---|---|
| 新 Provider 支持 | P1 | `OllamaFormat`、`LocalAiFormat`、`DeepSeekFormat` 等 |
| 新文件格式 | P2 | Markdown、Office 文档（docx/xlsx/pptx）、代码文件语法高亮元数据 |
| 流式加载 | P2 | 大文件（视频、长音频）的分块/流式加载支持 |
| 动态工具热插拔 | P2 | `ToolKit::unregister()` + 运行时替换工具；MCP 客户端工具列表热更新 |
| WebSocket 传输 | P2 | MCP 客户端/服务器支持 WebSocket 作为替代传输方式 |
| 工具链编排 | P2 | 在 toolkit 层提供简单的顺序/并行工具链辅助（不替代 Pandaria 工作流，但降低入门门槛） |

---

### 1.0.0 — 稳定版

**目标**：API 冻结，对外承诺向后兼容，标记生产就绪。

| 工作项 | 优先级 | 说明 |
|---|---|---|
| API 冻结 | P0 | 所有 `pub` trait 和类型在 1.0 后遵循 semver；breaking change 仅在 2.0 进行 |
| 迁移指南 | P0 | 从 0.x 到 1.0 的完整迁移指南（如需要） |
| 安全审计 | P1 | 对路径遍历、SSRF、MCP 输入校验等进行独立安全审查 |
| 生产就绪声明 | P0 | README 更新，明确标注生产可用；社区使用案例收集 |
| 长期支持 (LTS) 承诺 | P1 | 明确 1.0.x 的维护周期（如 12 个月安全更新） |

---

### 未定优先级（Backlog）

- **Wasm 支持**：评估核心 crate 在 `wasm32-unknown-unknown` 目标下的编译可行性
- **No-std 子集**：`pawbun-mcp-core` 可能是最小依赖的候选，可评估部分 no-std 支持
- **插件系统**：基于 `libloading` 的动态 `.so` / `.dll` 工具加载
- **GRPC 传输**：MCP 的 gRPC 变体支持（若社区标准演进至此）
