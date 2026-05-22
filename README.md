# Pawbun

Pawbun 是 [Pandaria](https://github.com/0xnicholas/pandaria) 生态的 Rust workspace，为 Agent 系统提供工具注册、执行以及多模态文件处理能力。

包含 3 个 crate：

- **pawbun-toolkit** — Agent 工具集（类似 CrewAI Tools），含内置工具、MCP 客户端、过程宏
- **pawbun-toolkit-macros** — `#[pawbun_tool]` 属性宏，自动生成 `Tool` 样板代码
- **pawbun-files** — 多模态文件处理（类似 CrewAI Files），支持多种媒体类型的统一加载与 LLM Provider 格式化

## 特性

- **工具注册/执行** — 统一的 `Tool` trait + `ToolKit` 注册中心，支持同步与异步执行
- **MCP 客户端** — 完整的 Model Context Protocol 客户端，支持 Stdio / SSE 两种传输方式
- **过程宏** — `#[pawbun_tool]` 自动生成 `name()`、`description()`、`parameters()` 方法
- **沙箱安全** — 路径遍历防护（`canonicalize` + 前缀校验）+ SSRF 防护（阻止内网/本地地址）
- **多 Provider 格式化** — 将加载的文件内容格式化为 OpenAI / Anthropic / Gemini / Azure OpenAI 的消息块
- **文件约束** — 大小、尺寸、媒体类型检查，自动降级策略

## 项目结构

```
Pawbun/
├── Cargo.toml
├── crates/
│   ├── pawbun-toolkit/          # Agent 工具注册与执行
│   ├── pawbun-toolkit-macros/   # #[pawbun_tool] 过程宏
│   └── pawbun-files/            # 多模态文件处理
├── docs/
│   ├── specs/
│   └── plans/
```

---

## pawbun-toolkit

Agent 工具集，提供核心 `Tool` trait 和 `ToolKit` 注册中心，使 Agent 能够以结构化的方式发现和调用各种能力。

### 核心抽象

| 类型 / Trait | 说明 |
|---|---|
| `Tool` | 所有工具实现的基础 trait：`name()`、`description()`、`parameters()`、`execute()` |
| `AsyncTool` | 异步工具扩展 trait，增加 `execute_async()` |
| `ToolKit` | 默认注册中心（`BTreeMap<String, Arc<dyn Tool>>`），实现 `ToolRegistry` + `ToolExecutor` + `AsyncToolExecutor` |
| `ToolRegistry` | 工具发现：`get()`、`list()`、`descriptions()` |
| `ToolExecutor` | 同步调用：`execute(name, input_json)` |
| `AsyncToolExecutor` | 异步调用：`execute_async(name, input_json)` |
| `ToolResult` | 统一返回类型：`success`、`content`、`metadata`、`elapsed_ms` |
| `BlockingExecutor` | 可插拔的阻塞执行策略（默认 `TokioExecutor`） |

### 内置工具

| 工具名称 | 输入参数 | 说明 | Feature |
|---|---|---|---|
| `file_read` | `path` | 读取文件，支持沙箱路径解析和大小限制 | — |
| `file_write` | `path`, `content` | 写入文件，自动创建父目录，TOCTOU 防护 | — |
| `directory_list` | `path` | 列出目录内容，返回 JSON 数组（含类型/大小） | — |
| `web_fetch` | `url`, `max_length`? | HTTP 请求获取页面内容，异步执行 | `http` |
| `web_search` | `query`, `max_results`? | 调用可配置的搜索 API | `http` |
| `csv_query` | `csv`, `has_header`?, `columns`?, `limit`? | 查询 CSV 数据 | `csv` |
| `json_query` | `data`, `query` | JSONPath 查询 | `jsonpath` |
| `code_execute` | `code`, `language`?, `timeout_ms`? | 代码执行（占位，待外部沙箱集成） | — |
| `embedding` | `text`, `model`? | 文本嵌入（占位，待外部服务集成） | — |
| `vision` | `image`, `prompt`? | 视觉分析（占位，待外部模型集成） | — |

### MCP 模块

`pawbun-toolkit` 内置 MCP (Model Context Protocol) 客户端，可以连接外部 MCP 服务器，将其提供的工具代理为本地 `Tool` 实现：

- **传输方式**：`StdioTransport`（子进程 stdin/stdout）和 `SseTransport`（HTTP SSE，含握手和重试）
- **协议**：JSON-RPC 2.0 + MCP 消息类型
- **动态工具**：`DynamicTool` 将 MCP 工具包装为 `Tool` trait 实现
- **Schema 转换**：自动将 MCP 的 JSON Schema `input_schema` 转为 `ToolParameter` 列表
- **安全**：SSE 传输内置 SSRF 防护

### Feature Flags

| Feature | 说明 |
|---|---|
| `http` | 启用 `WebFetchTool`、`WebSearchTool` |
| `tokio` | 启用 `TokioExecutor`（异步上下文中的阻塞执行） |
| `schemars` | 启用 `ToolParameter::from_schema::<T>()` |
| `macros` | 启用 `#[pawbun_tool]` 属性宏 |
| `jsonpath` | 启用 `JsonQueryTool` |
| `csv` | 启用 `CsvQueryTool` |
| `tracing` | 工具执行 trace 打点 |
| `full` | 启用所有 feature |

### 快速开始

```toml
[dependencies]
pawbun-toolkit = { version = "0.1", features = ["full"] }
```

```rust
use pawbun_toolkit::{ToolKit, ToolExecutor, FileReadTool, FileWriteTool};

// 创建注册中心
let mut toolkit = ToolKit::new();

// 注册内置工具
toolkit.register(Box::new(FileReadTool::default()));
toolkit.register(Box::new(FileWriteTool::default()));

// 同步执行
let result = toolkit.execute("file_read", r#"{"path": "README.md"}"#).unwrap();
println!("{}", result.content);

let result = toolkit.execute(
    "file_write",
    r#"{"path": "output.txt", "content": "Hello Pawbun"}"#,
).unwrap();
println!("{}", result.content);
```

**使用 `#[pawbun_tool]` 宏定义自定义工具：**

```rust
use pawbun_toolkit::{Tool, ToolResult, ToolParameter, ToolError};
use pawbun_toolkit_macros::pawbun_tool;

#[derive(Debug)]
struct EchoTool;

#[pawbun_tool(name = "echo", description = "Echoes back the input message")]
impl Tool for EchoTool {
    fn execute(&self, input: serde_json::Value) -> Result<ToolResult, ToolError> {
        let msg = input["message"].as_str().unwrap_or("nothing");
        Ok(ToolResult::success(msg))
    }
}
```

---

## pawbun-toolkit-macros

过程宏 crate，提供 `#[pawbun_tool]` 属性宏，为 `impl Tool for Struct` 块自动生成 `name()`、`description()` 和 `parameters()` 方法。

### 宏参数

| 参数 | 必填 | 说明 |
|---|---|---|
| `name` | 是 | 工具名称字符串 |
| `description` | 是 | 工具描述字符串 |
| `crate` | 否 | 自定义 crate 路径（默认 `::pawbun_toolkit`） |

- 如果 impl 块中已存在 `name()` / `description()` / `parameters()` 方法，则保留用户定义版本，宏不会覆盖。
- 必须至少显式实现 `execute()` 方法。

---

## pawbun-files

多模态文件处理工具，提供统一的、类型安全的、可扩展的文件处理层。支持本地路径、URL 和内存字节三种来源的文本、图片、PDF、音频、视频文件。

### 四层架构

| 层 | 核心类型 | 职责 |
|---|---|---|
| **Type** | `File`, `MediaType`, `MediaContent` | 统一的文件/媒体表示 |
| **Source** | `FileSource::Local / Url / Bytes` | 抽象文件来源 |
| **Loader** | `FileLoader`, `AsyncFileLoader`, `DefaultFileLoader` | 读取、校验、解析、格式检测 |
| **Provider** | `ProviderFormat`, 4 个 Provider 实现 | 格式化为 LLM API 消息块 |

### 媒体类型

- **Text** — 纯文本
- **Image** — PNG, JPEG, GIF, WebP, SVG, BMP
- **Pdf** — PDF 文档
- **Audio** — MP3, WAV, OGG, AAC, FLAC
- **Video** — MP4, WebM, AVI, MOV
- **Binary** — 通用二进制

### Provider 支持

| Provider | 文本 | 图片 | PDF | 音频 | 视频 | 传输方式 |
|---|---|---|---|---|---|---|
| `OpenAiFormat` | ✅ | ✅ data_uri | — | — | — | Inline / URL |
| `AnthropicFormat` | ✅ | ✅ base64 | ✅ document | — | — | Inline |
| `GeminiFormat` | ✅ | ✅ inline_data | ✅ inline_data | ✅ inline_data | ✅ inline_data | Inline |
| `AzureOpenAiFormat` | ✅ | ✅ data_uri | — | ✅ input_audio | — | Inline / URL |

### 文件约束

```rust
use pawbun_files::{File, FileConstraints, OverflowMode, MediaType, ImageFormat};

let file = File::from_path("./image.png")
    .with_constraints(FileConstraints {
        max_size_bytes: Some(5 * 1024 * 1024),       // 最大 5MB
        allowed_media_types: Some(vec![
            MediaType::Image(ImageFormat::Png),
            MediaType::Image(ImageFormat::Jpeg),
        ]),
        overflow_mode: OverflowMode::Strict,           // 超限则报错
        ..Default::default()
    });
```

### Feature Flags

| Feature | 说明 |
|---|---|
| `url-source` | 启用 HTTP 下载 `FileSource::Url` |
| `image-meta` | 启用图片尺寸提取和 `downgrade_image()` 降级 |
| `tokio` | 启用异步文件 I/O 和并行批量加载 |
| `tracing` | 加载和格式化 trace 打点 |
| `full` | 启用所有 feature |

### 快速开始

```toml
[dependencies]
pawbun-files = { version = "0.1", features = ["full"] }
```

**同步加载 + Provider 格式化：**

```rust
use pawbun_files::{File, DefaultFileLoader, FileLoader, OpenAiFormat, ProviderFormat};

let loader = DefaultFileLoader::new();
let file = File::from_path("./chart.png").with_key("sales_chart");

let loaded = loader.load(&file).expect("load");
let block = OpenAiFormat.format_content(&loaded.content).expect("format");

// block 可直接嵌入 OpenAI API 的 messages
```

**异步加载：**

```rust
use pawbun_files::{File, DefaultFileLoader, AsyncFileLoader};

let loader = DefaultFileLoader::new();
let file = File::from_path("./report.pdf");

let loaded = loader.load_async(&file).await.expect("load async");
```

**Provider 切换：**

```rust
use pawbun_files::{File, DefaultFileLoader, FileLoader, GeminiFormat, ProviderFormat};

let loader = DefaultFileLoader::new();
let file = File::from_path("./diagram.png");
let loaded = loader.load(&file).unwrap();

// 无需修改加载逻辑，只需切换 Provider
let block = GeminiFormat.format_content(&loaded.content).unwrap();
```

**沙箱加载：**

```rust
use pawbun_files::{File, DefaultFileLoader, FileLoader};

// 限制只能读取 /app/data 下的文件
let loader = DefaultFileLoader::with_base_dir("/app/data");
let file = File::from_path("./report.txt");
let loaded = loader.load(&file).expect("load");  // 路径遍历攻击将被拒绝
```

**构造 File：**

```rust
use pawbun_files::{File, MediaType, ImageFormat};
use bytes::Bytes;

// 从本地路径（自动检测类型）
let f1 = File::from_path("./report.pdf");

// 从 URL
let f2 = File::from_url("https://example.com/chart.png");

// 从内存字节
let data = Bytes::from_static(b"hello world");
let f3 = File::from_bytes(data, "note.txt");

// 显式指定媒体类型
let f4 = File::from_path("./data.bin")
    .with_media_type(MediaType::Image(ImageFormat::Png));
```

---

## 构建与开发

```bash
# 检查整个 Workspace
cargo check --workspace

# 运行所有测试
cargo test --workspace

# 带 feature 测试
cargo test --workspace --all-features

# 生成文档
cargo doc --workspace --no-deps --open
```

## 参考

- [Pandaria](https://github.com/0xnicholas/pandaria) — Pawbun 所属的 Agent 框架生态
- [CrewAI Tools](https://github.com/crewAIInc/crewAI/tree/main/lib/crewai-tools)
- [Model Context Protocol](https://modelcontextprotocol.io/)

## License

MIT OR Apache-2.0
