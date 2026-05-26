# Pawbun 版本记录

> 本文件记录 Pawbun workspace 各 crate 的版本状态与发布历史。
>
> 版本号遵循 [SemVer](https://semver.org/lang/zh-CN/)。

---

## 当前版本：0.1.0

**发布日期**：待定（开发分支已完成，待打 tag）

**Workspace 版本**：`0.1.0`

| Crate | 版本 | 状态 | 说明 |
|---|---|---|---|
| `pawbun-toolkit` | 0.1.0 | ✅ 可用 | Agent 工具注册与执行核心 |
| `pawbun-toolkit-macros` | 0.1.0 | ✅ 可用 | `#[pawbun_tool]` 过程宏 |
| `pawbun-files` | 0.1.0 | ✅ 可用 | 多模态文件加载与 Provider 格式化 |
| `pawbun-mcp-core` | 0.1.0 | ✅ 可用 | MCP 协议类型与传输抽象 |
| `pawbun-mcp-server` | 0.1.0 | ✅ 可用 | MCP 服务器（stdio / SSE）|

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
- ⚠️ 占位工具（接口已定义，执行返回错误，需外部集成）：
  - `CodeExecuteTool` — 需外部沙箱（Docker / Firejail / e2b）
  - `EmbeddingTool` — 需外部 embedding 服务
  - `VisionTool` — 需外部多模态模型
- ✅ MCP 客户端（Stdio + SSE 传输，`DynamicTool` 代理远程工具）

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

- ✅ `McpServer` + `McpServerBuilder`
- ✅ `RequestHandler`：initialize 状态机 + 方法路由
- ✅ 支持方法：`initialize` / `notifications/initialized` / `tools/list` / `tools/call`
- ✅ Stdio 传输（生产就绪）
- ✅ SSE 传输（`http` feature，基于 axum）
- ✅ `FileLoader` → MCP Tool bridge（`file_read` / `file_list`）
- ✅ 工具去重：用户注册工具优先于 bridge 工具

---

## 质量指标

| 指标 | 数值 |
|---|---|
| 编译状态 | ✅ 零错误 |
| Clippy | ✅ 零警告 (`-D warnings`) |
| 单元测试 | ✅ 176 passed / 0 failed |
| 文档测试 | ✅ 32 passed / 0 failed |
| 代码行数 | ~15,100 行 Rust |
| 测试覆盖率 | 核心模块全覆盖（trait、工具、loader、provider、handler、transport） |

---

## 已知限制

1. **占位工具**：`CodeExecuteTool`、`EmbeddingTool`、`VisionTool` 仅为接口占位，调用 `execute` 会返回错误提示，需外部服务集成后方可使用。此为设计意图。
2. **tracing 深度集成**：`tracing` feature 已存在，但未在所有关键路径加 `#[instrument]`。
3. **性能基准**：尚未建立正式的 benchmark 基线。
4. **无 CHANGELOG**：首次发布，尚无历史版本记录。

---

## 版本历史

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

### 0.2.0 — 生态集成

**目标**：降低占位工具的集成门槛，让 Pawbun 与外部服务（沙箱、LLM、Embedding）顺畅对接。

| 工作项 | 优先级 | 说明 |
|---|---|---|
| `CodeExecuteTool` 适配器示例 | P0 | 提供 Docker 沙箱适配器示例 crate（`pawbun-sandbox-docker` 或示例代码），演示如何将占位转为可用 |
| `VisionTool` 适配器示例 | P0 | 提供 OpenAI / Anthropic 视觉 API 的适配器示例 |
| `EmbeddingTool` 适配器示例 | P1 | 提供 OpenAI embedding API 或 `fastembed-rs` 的适配器示例 |
| 错误上下文增强 | P1 | 在 `ToolError` / `LoadError` / `McpServerError` 中加入 `#[source]` 链式错误，提升调试体验 |
| `pawbun-mcp-server` 配置化 | P1 | 支持通过 `McpServerBuilder` 自定义协议版本、capabilities、CORS（SSE）等 |
| SSE 传输稳定性 | P1 | 增加重连逻辑、心跳检测、连接池管理 |

**验收标准**：
- `cargo test --workspace --all-features` 全绿
- `cargo clippy --workspace --all-features -- -D warnings` 零警告
- 示例代码可在 README 中直接复制运行

---

### 0.3.0 — 性能、API 审计与文档

**目标**：建立性能基线，精简公共 API，成为社区可信赖的依赖。

| 工作项 | 优先级 | 说明 |
|---|---|---|
| Criterion 基准测试 | P0 | 填充现有 `crates/pawbun-toolkit/benches/toolkit.rs`：工具注册开销、注册表查找、序列化/反序列化、文件加载吞吐 |
| 基准目标 | P0 | 同步工具调用 overhead < 1μs；注册表查找 O(1) 实测确认 |
| `pub` API 审计 | P0 | 审查所有 `pub` 项，移除不必要的暴露；确保 semver 合规 |
| 模块重组织评估 | P1 | 评估是否将 `pawbun-toolkit::mcp` 下的客户端代码进一步解耦 |
| 文档增强 | P1 | 增加 `examples/` 目录（每个 crate 至少 2 个可运行示例）、cookbook 风格指南 |
| 兼容性测试 | P1 | 在 CI 中测试不同 feature 组合（最小依赖集 vs `full`） |
| 依赖精简 | P2 | 评估 `image` crate 等重型依赖是否可 feature-gate 得更细 |

**验收标准**：
- 基准测试报告纳入版本发布说明
- `cargo public-api`（如有）或人工审计确认无意外 breaking change
- docs.rs 上所有 crate 文档评分 A+

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
