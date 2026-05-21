# pawbun-files Specification

> Version: 0.1.0-draft  
> Status: Design  
> Date: 2026-05-20

---

## 1. 目标与范围

`pawbun-files` 为 Pandaria 生态的 Agent 提供**统一、类型安全、可扩展**的多模态文件输入处理层。设计参考 [CrewAI Files](https://docs.crewai.com/en/concepts/files)，但以 Rust 的零成本抽象和强类型为核心优势。

### 1.1 核心目标

- 提供统一的 `File` 类型，屏蔽文件来源（本地路径 / URL / 内存字节）与媒体类型的差异。
- 支持多模态内容表示（文本、图片、PDF、音频、视频），并携带结构化元数据。
- 提供同步与异步文件加载能力，兼容不同运行时。
- 为不同 LLM Provider 自动生成最优传输格式（Base64 / URL / Upload API）。
- 内置文件约束管理（大小、格式、尺寸），防止向 Provider 发送超限内容。
- 与 `pawbun-toolkit` 深度集成，工具链可直接消费和产出 `File` / `MediaContent`。

### 1.2 非目标

- 不实现具体的 LLM 客户端或 HTTP 调用逻辑（由 Pandaria 其他模块或 Provider 适配层负责）。
- 不强制绑定任何异步运行时（如 tokio），异步支持基于标准 `Future`。
- 不替代操作系统底层 IO，而是提供面向 Agent 场景的高层抽象。

---

## 2. 架构概述

```
┌──────────────────────────────────────────────────────────────────────┐
│  项目外：Pandaria Agent / Workflow / pawbun-toolkit                  │
│  - Agent 通过 toolkit 调用文件工具                                   │
│  - Workflow 编排多步骤文件处理                                        │
│  - 用户代码直接构造 File 并传入 LLM 请求                             │
└───────────────────────────┬──────────────────────────────────────────┘
                            │
┌───────────────────────────▼──────────────────────────────────────────┐
│  pawbun-files（核心库）                                               │
│  ┌─────────────────────────────────────────────────────────────────┐ │
│  │  File（统一入口）                                                │ │
│  │  - key: Option<String>（模板引用键名）                          │ │
│  │  - source: FileSource（Path / Url / Bytes）                     │ │
│  │  - media_type: MediaType（自动检测或显式指定）                   │ │
│  │  - constraints: FileConstraints（大小/格式限制）                │ │
│  │  - metadata: FileMetadata（用户预设/加载后填充）                │ │
│  └─────────────────────────────────────────────────────────────────┘ │
│                            │                                         │
│          ┌─────────────────┼─────────────────┐                       │
│          ▼                 ▼                 ▼                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐               │
│  │  FileLoader  │  │  FileSource  │  │ ProviderFmt  │               │
│  │  trait       │  │  enum        │  │  trait       │               │
│  └──────────────┘  └──────────────┘  └──────────────┘               │
│          │                 │                 │                       │
│          ▼                 ▼                 ▼                       │
│  ┌──────────────────────────────────────────────────────────────┐   │
│  │  MediaContent（富枚举）                                       │   │
│  │  - Text(String)                                               │   │
│  │  - Image { bytes, width, height, format }                     │   │
│  │  - Pdf { bytes, pages, text_preview }                         │   │
│  │  - Audio { bytes, duration, sample_rate }                     │   │
│  │  - Video { bytes, duration, thumbnail }                       │   │
│  │  - Binary(Bytes)                                              │   │
│  └──────────────────────────────────────────────────────────────┘   │
└──────────────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌──────────────────────────────────────────────────────────────────────┐
│  Provider 适配输出（JSON / Base64 / URL）                             │
│  - OpenAI messages.content.image_url                                │
│  - Anthropic messages.content.source                                │
│  - Gemini inline_data                                               │
└──────────────────────────────────────────────────────────────────────┘
```

### 2.1 四层架构

| 层级 | 职责 | 代表类型 |
|---|---|---|
| **类型层** | 定义文件与内容的统一表示 | `File`, `MediaType`, `MediaContent` |
| **源层** | 抽象文件来源，屏蔽获取方式差异 | `FileSource`, `LocalSource`, `UrlSource`, `BytesSource` |
| **加载层** | 从来源读取、验证并解析为元数据 | `FileLoader`, `AsyncFileLoader` |
| **适配层** | 将内容格式化为 Provider 所需结构 | `ProviderFormat`, `OpenAiFormat`, `AnthropicFormat` |

---

## 3. 核心概念与接口

### 3.1 File（统一文件句柄）

`File` 是用户和 Agent 操作文件的统一入口，类似 CrewAI 的 `File` 类，但显式分离了来源与内容。

> **序列化提示**：`File` 实现 `Serialize` / `Deserialize` 主要用于进程内缓存与任务队列传递。
> `FileSource::Local` 包含本地路径，跨机器序列化后路径大概率失效，跨网络传输时建议优先使用 `FileSource::Url` 或 `FileSource::Bytes`。

```rust
use std::path::Path;

/// 统一文件句柄，表示一个待加载或已加载的多模态文件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    /// 模板引用键名（如 `"sales_chart"`），供 Agent prompt 模板 `{sales_chart}` 匹配。
    /// 若为 `None`，表示不由模板引擎引用。
    ///
    /// **消费方**：`key` 由 Pandaria 上层的 Agent / Workflow 模板系统消费，
    /// `pawbun-files` 本身不解析模板。工具链在传递 `File` 时应保留此字段。
    pub key: Option<String>,

    /// 文件来源（本地路径 / URL / 内存字节）。
    pub source: FileSource,

    /// 媒体类型。若为 `None`，由 `FileLoader` 自动检测。
    pub media_type: Option<MediaType>,

    /// 用户指定的约束条件（大小、格式白名单等）。
    pub constraints: FileConstraints,

    /// 文件元数据（文件名、MIME 类型、修改时间等）。
    /// 创建时可由用户预设（如 `from_bytes` 自动填充 `name`）。
    ///
    /// **与加载的关系**：`FileLoader::load()` 返回的 `LoadedContent.metadata` 包含解析后的实际元数据。
    /// 由于 `FileLoader` 为纯函数（不修改输入 `File`），调用方需自行将加载后的元数据合并回 `File`：
    /// ```rust
    /// let loaded = loader.load(&file)?;
    /// file.metadata = loaded.metadata; // 用实际元数据覆盖预设值
    /// ```
    pub metadata: FileMetadata,
}

impl File {
    /// 从本地路径创建文件，自动推断媒体类型。
    /// 不验证路径是否存在或可读，失败推迟到 `FileLoader::load()` 阶段。
    pub fn from_path<P: AsRef<Path>>(path: P) -> Self;

    /// 从 URL 创建文件。
    /// 不验证 URL 是否可达或有效，失败推迟到 `FileLoader::load()` 阶段。
    pub fn from_url(url: impl Into<String>) -> Self;

    /// 从内存字节创建文件。
    /// `hint` 为文件名或扩展名，用于推断媒体类型并填充 `metadata.name`
    ///（如 `"chart.png"` → `media_type=Image(Png)`, `metadata.name="chart.png"`）。
    pub fn from_bytes(bytes: bytes::Bytes, hint: &str) -> Self;

    /// 设置模板引用键名。
    pub fn with_key(mut self, key: impl Into<String>) -> Self;

    /// 设置媒体类型（覆盖自动检测）。
    pub fn with_media_type(mut self, ty: MediaType) -> Self;

    /// 设置约束条件。
    pub fn with_constraints(mut self, c: FileConstraints) -> Self;
}

/// 文件元数据。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FileMetadata {
    /// 原始文件名（不含路径）。
    pub name: Option<String>,
    /// MIME 类型，如 `image/png`。
    pub mime_type: Option<String>,
    /// 文件大小（字节）。使用 `u64` 以支持 32 位系统及大文件（>4GB）。
    pub size_bytes: Option<u64>,
    /// 最后修改时间。
    pub modified_at: Option<std::time::SystemTime>,
}
```

#### 设计决策：为什么用 `File` 而非类型特化的 `ImageFile` / `PdfFile`？

| 方案 | 优点 | 缺点 | 结论 |
|---|---|---|---|
| **类型特化**（CrewAI 风格：`ImageFile`, `PDFFile`） | API 直观，IDE 自动补全友好 | 文件类型增多时类型爆炸；自动检测后需显式转换 | ❌ 放弃 |
| **统一 `File` + `MediaType`** | 来源与类型解耦；自动检测无需转换；扩展新类型只需加枚举变体 | 媒体类型信息在运行时携带 | ✅ 采纳 |

---

### 3.2 FileSource（文件来源抽象）

```rust
/// 文件数据来源。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FileSource {
    /// 本地文件系统路径。
    Local { path: std::path::PathBuf },
    /// 远程 URL（HTTP/HTTPS）。
    Url { url: String },
    /// 内存中的字节数据（引用计数，cheap clone）。
    Bytes { data: bytes::Bytes },
}

impl FileSource {
    /// 尝试返回一个可用于标识此来源的字符串（文件名或 URL 最后一段）。
    pub fn hint(&self) -> Option<String>;
}
```

- **Local**：通过 `std::fs` 读取，支持路径安全校验（防止路径遍历）。
- **Url**：通过 HTTP 客户端下载（需启用 `reqwest` feature）。URL 可直接透传给支持 URL 引用的 Provider。
- **Bytes**：已存在于内存中的数据，常用于从 API 响应或缓存构造文件。
> 使用 `bytes::Bytes`（引用计数），`File` 的 `Clone` 不会复制底层数据。

---

### 3.3 MediaType（媒体类型）

当前已实现的骨架扩展为支持更细粒度的子类型：

```rust
/// 媒体类型枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum MediaType {
    /// 纯文本或结构化文本（Markdown, JSON, CSV 等）。
    Text,
    /// 图片，携带具体格式信息。
    Image(ImageFormat),
    /// PDF 文档。
    Pdf,
    /// 音频数据。
    Audio(AudioFormat),
    /// 视频数据。
    Video(VideoFormat),
    /// 未知或未识别的二进制格式。
    Binary,
}

/// 图片格式子类型。
/// 未知图片格式不通过 `Other` 表示，而归入 `MediaType::Binary`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ImageFormat {
    Png,
    Jpeg,
    Gif,
    Webp,
    Svg,
    Bmp,
}

/// 音频格式子类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AudioFormat {
    Mp3,
    Wav,
    Ogg,
    Aac,
    Flac,
}

/// 视频格式子类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum VideoFormat {
    Mp4,
    Webm,
    Avi,
    Mov,
}

impl MediaType {
    /// 从文件扩展名推断媒体类型。
    pub fn from_extension(ext: &str) -> Option<Self>;

    /// 返回此媒体类型的默认 MIME 类型。
    /// 对于携带子类型的变体（`Image` / `Audio` / `Video`），此方法是子类型 `mime_type()` 的便捷封装。
    /// `Text` 返回 `"text/plain"`，`Pdf` 返回 `"application/pdf"`，`Binary` 返回 `None`。
    pub fn mime_type(&self) -> Option<&'static str>;
}

impl ImageFormat {
    /// 返回此图片格式的 MIME 类型。
    pub fn mime_type(&self) -> &'static str;
}

impl AudioFormat {
    /// 返回此音频格式的 MIME 类型。
    pub fn mime_type(&self) -> &'static str;
}

impl VideoFormat {
    /// 返回此视频格式的 MIME 类型。
    pub fn mime_type(&self) -> &'static str;
}
```

**迁移说明**：`MediaType::Image` 从简单变体变为携带 `ImageFormat` 的变体，这是 **breaking change**（旧代码 `MediaType::Image` 无法直接匹配新变体）。
旧 `FileHandler::supported_types` 返回 `vec![MediaType::Image]` 应迁移为 `vec![MediaType::Image(ImageFormat::Png), MediaType::Image(ImageFormat::Jpeg), ...]` 或使用 `matches!(ty, MediaType::Image(_))` 通配匹配。

**`FileHandler` 演进**：当前 `lib.rs` 的 `FileHandler` trait 将被 `FileLoader` 取代。由于 `MediaType` 与 `MediaContent` 均为 breaking change，`FileHandler` 无法提供无缝兼容。MVP 阶段直接移除 `FileHandler`（不保留 deprecated 兼容层），迁移指南见附录。

---

### 3.4 MediaContent（富枚举内容表示）

当前骨架扩展为携带各媒体类型的结构化元数据：

```rust
/// 统一的多模态内容表示。
///
/// 实现 `Serialize` / `Deserialize` 时，所有 `bytes::Bytes` 字节字段通过内部模块编码为 Base64 字符串，
/// 避免默认数字数组序列化导致的体积膨胀。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MediaContent {
    /// 文本内容。
    Text(TextContent),
    /// 图片内容，携带像素尺寸与格式。
    Image(ImageContent),
    /// PDF 文档内容。
    Pdf(PdfContent),
    /// 音频内容，携带时长与采样率。
    Audio(AudioContent),
    /// 视频内容，携带时长与缩略图。
    Video(VideoContent),
    /// 未知或未解析的二进制内容。
    Binary(BinaryContent),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextContent {
    pub text: String,
    /// 文本编码（通常 UTF-8）。
    pub encoding: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageContent {
    /// 原始图片字节（序列化时为 Base64 字符串；内部为引用计数，cheap clone）。
    #[serde(with = "base64_bytes")]
    pub bytes: bytes::Bytes,
    pub format: ImageFormat,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdfContent {
    /// 原始 PDF 字节（序列化时为 Base64 字符串；内部为引用计数，cheap clone）。
    #[serde(with = "base64_bytes")]
    pub bytes: bytes::Bytes,
    /// 页数（若已解析）。
    pub pages: Option<usize>,
    /// 文本预览（前 N 字符），用于调试与日志。
    pub text_preview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioContent {
    /// 原始音频字节（序列化时为 Base64 字符串；内部为引用计数，cheap clone）。
    #[serde(with = "base64_bytes")]
    pub bytes: bytes::Bytes,
    pub format: AudioFormat,
    /// 音频时长（秒）。
    pub duration: Option<f64>,
    /// 采样率（Hz）。
    pub sample_rate: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoContent {
    /// 原始视频字节（序列化时为 Base64 字符串；内部为引用计数，cheap clone）。
    #[serde(with = "base64_bytes")]
    pub bytes: bytes::Bytes,
    pub format: VideoFormat,
    /// 视频时长（秒）。
    pub duration: Option<f64>,
    /// 视频缩略图（第一帧图片，可选）。
    pub thumbnail: Option<ImageContent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BinaryContent {
    /// 原始二进制字节（序列化时为 Base64 字符串；内部为引用计数，cheap clone）。
    #[serde(with = "base64_bytes")]
    pub bytes: bytes::Bytes,
    /// 若已知，用户提供的媒体类型推测。
    pub guessed_type: Option<MediaType>,
}

impl MediaContent {
    /// 返回内容的原始字节（若有）。
    pub fn as_bytes(&self) -> Option<&[u8]>;

    /// 返回文本内容（仅 Text 变体）。
    pub fn as_text(&self) -> Option<&str>;

    /// 返回此内容的媒体类型。
    pub fn media_type(&self) -> MediaType;

    /// 返回内容大小（字节）。
    pub fn size_bytes(&self) -> u64;
}

/// **序列化策略**：`MediaContent` 实现 `Serialize` 时，所有 `bytes::Bytes` 字节字段
/// 通过内部自定义 serde 模块 `base64_bytes` 编码为 JSON 字符串（如 `"iVBORw0KGgo..."`），
/// 避免默认的数字数组 `[1,2,3,...]` 导致体积膨胀。
/// `base64_bytes` 模块基于 `base64` 与 `bytes` crate 实现，支持 `Bytes` 类型的编解码。
/// 反序列化时自动解码为 `bytes::Bytes`。
///
/// 工具链内部传递优先使用 `MediaContent` 本身（零拷贝），
/// 仅在跨进程/网络传输时序列化。
```

#### 设计决策：MediaContent 的演进路径

| 维度 | 当前骨架 | Spec 目标 | 兼容性 |
|---|---|---|---|
| 图片 | `Binary(Vec<u8>)` | `Image { bytes, width, height, format }` | Breaking |
| PDF | 无 | `Pdf { bytes, pages, text_preview }` | 新增 |
| 音频 | 无 | `Audio { bytes, duration, sample_rate }` | 新增 |
| 视频 | 无 | `Video { bytes, duration, thumbnail }` | 新增 |

**Breaking Change**：旧代码 `MediaContent::Binary(bytes)` 无法直接匹配新变体（`Binary` 已变为 `Binary(BinaryContent)`）。迁移时需将 `MediaContent::Binary(bytes)` 改为 `MediaContent::Binary(BinaryContent { bytes, guessed_type: None })`，或优先使用新的结构化变体（`Image` / `Pdf` / `Audio` / `Video`）。

---

### 3.5 FileLoader（加载层）

采用 **双 trait 设计**：`FileLoader`（同步）+ `AsyncFileLoader`（异步），与 `pawbun-toolkit` 的 `Tool` / `AsyncTool` 保持一致。

```rust
use std::path::Path;

/// 加载结果，包含内容、元数据及可能的警告。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadedContent {
    pub content: MediaContent,
    pub metadata: FileMetadata,
    /// 加载及后续处理中产生的非致命警告。
    /// 例如：约束检查触发 `OverflowMode::Warn` 时的提示信息。
    pub warnings: Vec<String>,
}

/// 同步文件加载器。
///
/// 负责将 `FileSource` 读取并解析为 `MediaContent`。
/// **纯函数设计**：不修改输入 `File`，所有结果通过 `LoadedContent` 返回。
pub trait FileLoader: std::fmt::Debug + Send + Sync {
    /// 加载单个文件，返回 `LoadedContent`。
    ///
    /// 实现应自动检测媒体类型（若 `file.media_type` 为 `None`），
    /// 解析结构化元数据（图片尺寸、PDF 页数等）。
    ///
    /// **类型冲突处理**：若用户显式指定了 `file.media_type` 且与实际检测结果不符，
    /// 返回 `LoadError::TypeMismatch`。若 `file.media_type` 为 `None`，loader 直接覆盖为检测结果，不报错。
    ///
    /// **注意**：`FileLoader` 为纯加载职责，不校验 `FileConstraints`。
    /// 约束检查由调用方通过 `FileConstraints::check()` 显式执行。
    fn load(&self, file: &File) -> Result<LoadedContent, LoadError>;

    /// 批量加载多个文件，默认顺序执行。
    /// 返回结果顺序与输入 `files` 顺序严格一致，每个结果关联对应的 `&File`，
    /// 便于调用方在失败时定位具体文件。
    /// 实现可覆盖为并发执行（如 URL 批量下载），但仍须保证输出顺序与输入一一对应。
    fn load_batch(&self, files: &[File]) -> Vec<(&File, Result<LoadedContent, LoadError>)> {
        files.iter().map(|f| (f, self.load(f))).collect()
    }

    /// 仅获取文件的元数据，不读取完整内容（轻量操作）。
    fn metadata(&self, file: &File) -> Result<FileMetadata, LoadError>;
}

/// 异步文件加载器扩展 trait。
///
/// **设计权衡**：`AsyncFileLoader` 继承 `FileLoader`，要求异步实现者同时提供同步 `load()`。
/// 这保证了 API 的一致性，但对于纯异步后端（如 S3）可能需要阻塞适配。
/// 若场景仅需异步，可只实现此 trait 并通过默认适配器暴露同步接口。
///
/// **MSRV**：需要 Rust 1.75+（原生 `async fn` in trait）。
/// 若需 `dyn AsyncFileLoader`（trait object），当前 Rust 版本需手动将返回类型 boxing 为
/// `Pin<Box<dyn Future<Output = ...> + Send>>`，或仍使用 `async-trait` crate。
pub trait AsyncFileLoader: FileLoader {
    async fn load_async(&self, file: &File) -> Result<LoadedContent, LoadError>;
    async fn load_batch_async(&self, files: &[File]) -> Vec<(&File, Result<LoadedContent, LoadError>)>;
    async fn metadata_async(&self, file: &File) -> Result<FileMetadata, LoadError>;
}

/// 默认加载器实现，覆盖所有内置文件源。
///
/// 通过 `with_base_dir` 可设置沙箱根目录，未设置时使用当前工作目录。
#[derive(Debug, Clone, Default)]
pub struct DefaultFileLoader {
    pub base_dir: Option<std::path::PathBuf>,
}

impl DefaultFileLoader {
    pub fn new() -> Self;
    pub fn with_base_dir<P: Into<std::path::PathBuf>>(base_dir: P) -> Self;
}

impl FileLoader for DefaultFileLoader {
    fn load(&self, file: &File) -> Result<LoadedContent, LoadError> {
        // 1. 读取源（本地 fs / HTTP / 内存）
        // 2. 自动检测媒体类型（基于扩展名 / Magic Bytes / MIME）
        // 3. 解析结构化元数据（图片尺寸、PDF 页数等）
        // 4. 返回 LoadedContent（不校验 FileConstraints）
    }
}
```

---

### 3.6 LoadError（统一错误类型）

```rust
use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum LoadError {
    #[error("IO error: {0}")]
    Io(String),

    #[error("unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("network error: {0}")]
    Network(String),

    #[error("path traversal detected: {0}")]
    PathTraversal(String),

    #[error("media type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: MediaType, actual: MediaType },
}
```

---

## 4. Provider 适配层

### 4.1 设计目标

不同 LLM Provider 对多模态输入的 API 结构差异很大：
- **OpenAI**：`content: [{type: "image_url", image_url: {url: "data:image/png;base64,..."}}]`
- **Anthropic**：`content: [{type: "image", source: {type: "base64", media_type: "image/png", data: "..."}}]`
- **Gemini**：`inline_data: {mime_type: "image/png", data: "..."}`

`ProviderFormat` trait 将 `MediaContent` 转换为 Provider 特定的请求结构。

```rust
use serde_json::Value;

/// Provider 格式化 trait。
///
/// 将 `MediaContent` 转换为特定 LLM Provider API 所需的 JSON 片段。
///
/// **边界说明**：此 trait 只负责"内容格式化"，不包含 HTTP 上传逻辑。
/// `UploadApi` 传输方式由外部 LLM client 负责（需要 stateful 会话和 HTTP 调用），
/// pawbun-files 仅提供 `constraints()` 供外部判断是否需要走 UploadApi。
pub trait ProviderFormat: std::fmt::Debug + Send + Sync {
    /// Provider 名称（如 `"openai"`, `"anthropic"`）。
    fn provider_name(&self) -> &str;

    /// 将单个 `MediaContent` 转换为 Provider 的 content block。
    fn format_content(&self, content: &MediaContent) -> Result<Value, FormatError>;

    /// 将 `File` 统一格式化为 Provider 内容块（高层便利方法）。
    ///
    /// 内部自动判断：
    /// 1. 若 `file.source` 为 `Url` 且 `format_reference` 返回 `Some`，直接使用引用格式。
    /// 2. 否则通过 `loader` 加载文件，再调用 `format_content`。
    ///
    /// 此方法将"引用 vs 加载"的判断逻辑收敛到 trait 内部，避免每个调用方重复实现。
    fn format_file(&self, file: &File, loader: &dyn FileLoader) -> Result<Value, FormatError> {
        if let Some(reference) = self.format_reference(file) {
            return Ok(reference);
        }
        let loaded = loader.load(file)
            .map_err(|e| FormatError::Load(e.to_string()))?;
        self.format_content(&loaded.content)
    }

    /// 将 `File`（未加载）转换为 Provider 支持的引用格式（如 URL）。
    ///
    /// 仅当 `FileSource::Url` 且 Provider 支持 URL 引用时返回 `Some(Value)`。
    /// 其他情况返回 `None`，调用方需先 `load` 后调用 `format_content`。
    fn format_reference(&self, file: &File) -> Option<Value>;

    /// 返回此 Provider 对特定媒体类型的约束。
    fn constraints(&self, media_type: MediaType) -> ProviderConstraints;
}

/// Provider 对文件内容的约束。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConstraints {
    /// 单文件最大字节数。使用 `u64` 以支持 32 位系统及大文件。
    pub max_size_bytes: u64,
    /// 单次请求最大文件数。
    pub max_files_per_request: usize,
    /// 支持的 MIME 类型白名单。
    pub supported_mime_types: Vec<String>,
}

/// 格式化错误。
#[derive(Error, Debug, Clone)]
pub enum FormatError {
    #[error("unsupported media type for {provider}: {media_type}")]
    UnsupportedMediaType { provider: String, media_type: MediaType },
    #[error("content too large for {provider}: {size} bytes")]
    ContentTooLarge { provider: String, size: u64 },
    #[error("serialization failed: {0}")]
    Serialization(String),
    #[error("load failed: {0}")]
    Load(String),
}
```

### 4.2 传输策略

```rust
/// 文件传输方式。
///
/// **注意**：`UploadApi` 仅作为策略标识，实际 HTTP 上传由外部 LLM client 执行。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransmissionMethod {
    /// 内联 Base64 编码（文件嵌入请求体）。
    InlineBase64,
    /// 文件上传 API，返回 file_id 后在请求中引用。
    /// 由外部 LLM client 实现，pawbun-files 仅通过 `constraints()` 提示阈值。
    UploadApi,
    /// 直接透传 URL（文件源已是 URL 且 Provider 支持）。
    UrlReference,
}

impl ProviderFormat {
    /// 根据文件大小和来源自动选择最优传输方式。
    ///
    /// `loaded_size` 为实际加载后的内容大小（字节），优先于 `file.metadata.size_bytes`
    /// 使用，因为后者可能不准确或为空（如未启用 `image-meta`、解压后大小变化等）。
    ///
    /// 返回 `UploadApi` 时，调用方（外部 LLM client）需自行执行上传并获得 file_id。
    pub fn select_method(&self, file: &File, loaded_size: u64) -> TransmissionMethod {
        // 若 source 是 Url 且 Provider 支持 URL 引用 → UrlReference
        // 若文件大小 < Provider 内联阈值 → InlineBase64
        // 否则 → UploadApi（若 Provider 支持）或 InlineBase64（降级，可能失败）
    }
}
```

### 4.3 内置 Provider 支持矩阵

| Provider | Text | Image | PDF | Audio | Video | Inline Base64 | Upload API | URL Ref |
|---|---|---|---|---|---|---|---|---|
| **OpenAI** (chat) | ✓ | ✓ | ✗ | ✗ | ✗ | ✓ | ✓ (>5MB) | ✓ |
| **OpenAI** (responses) | ✓ | ✓ | ✓ | ✓ | ✗ | ✓ | ✓ | ✓ |
| **Anthropic** (claude-3) | ✓ | ✓ | ✓ | ✗ | ✗ | ✓ | ✓ (>5MB) | ✓ |
| **Gemini** (1.5+) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ (>20MB) | ✓ |
| **Azure OpenAI** | ✓ | ✓ | ✗ | ✓ | ✗ | ✓ | ✗ | ✓ |

> **注意**：Provider 支持矩阵基于 2026-05-20 的公开 API 文档。
> 各 Provider 的多模态支持能力更新频繁（如 Azure OpenAI 最新版已支持 PDF），
> 实际集成时请查阅对应 Provider 的最新官方文档。

### 4.4 示例：OpenAI ProviderFormat

```rust
#[derive(Debug, Clone, Default)]
pub struct OpenAiFormat;

impl ProviderFormat for OpenAiFormat {
    fn provider_name(&self) -> &str { "openai" }

    fn format_content(&self, content: &MediaContent) -> Result<Value, FormatError> {
        match content {
            MediaContent::Image(img) => {
                // 注：base64 crate 0.21+ 使用 `BASE64_STANDARD.encode(...)`
                let b64 = base64::encode(&img.bytes);
                let mime = img.format.mime_type()
                    .ok_or_else(|| FormatError::UnsupportedMediaType {
                        provider: "openai".into(),
                        media_type: content.media_type(),
                    })?;
                // 注：OpenAI Vision API 支持 `image_url.detail` 参数（"low" / "high" / "auto"）。
                // 当前版本未暴露此参数，未来可在 `FileConstraints` 或 `ProviderFormat` 扩展中增加。
                Ok(serde_json::json!({
                    "type": "image_url",
                    "image_url": {
                        "url": format!("data:{mime};base64,{b64}")
                    }
                }))
            }
            MediaContent::Text(txt) => Ok(serde_json::json!({
                "type": "text",
                "text": txt.text
            })),
            _ => Err(FormatError::UnsupportedMediaType {
                provider: "openai".into(),
                media_type: content.media_type(),
            }),
        }
    }

    fn constraints(&self, media_type: MediaType) -> ProviderConstraints {
        match media_type {
            MediaType::Image(_) => ProviderConstraints {
                max_size_bytes: 20 * 1024 * 1024,
                max_files_per_request: 10,
                supported_mime_types: vec!["image/png".into(), "image/jpeg".into(), "image/webp".into(), "image/gif".into()],
            },
            MediaType::Pdf => ProviderConstraints {
                max_size_bytes: 32 * 1024 * 1024,
                max_files_per_request: 1,
                supported_mime_types: vec!["application/pdf".into()],
            },
            _ => ProviderConstraints {
                max_size_bytes: u64::MAX,
                max_files_per_request: usize::MAX,
                supported_mime_types: vec![],
            },
        }
    }
}
```

#### 设计决策：Upload API 的边界

| 方案 | 优点 | 缺点 | 结论 |
|---|---|---|---|
| **ProviderFormat 包含 upload 方法** | 一站式解决 | 需引入 HTTP client + async + state（file_id），严重越界 | ❌ 放弃 |
| **Upload API 由外部 LLM client 实现** | pawbun-files 保持纯格式化职责；与具体 Provider SDK 解耦 | 调用方需自行处理上传流程 | ✅ 采纳 |

pawbun-files 通过 `select_method()` 建议 `"upload_api"`，外部 client 执行上传后将 `file_id` 注入请求体。
```

---

## 5. 约束与验证

### 5.1 FileConstraints

```rust
/// 文件约束条件。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileConstraints {
    /// 最大文件大小（字节）。`None` 表示无限制。使用 `u64` 以支持 32 位系统及大文件。
    pub max_size_bytes: Option<u64>,
    /// 允许的媒体类型白名单。`None` 表示全部允许。
    pub allowed_media_types: Option<Vec<MediaType>>,
    /// 图片最大宽度像素。
    pub max_image_width: Option<u32>,
    /// 图片最大高度像素。
    pub max_image_height: Option<u32>,
    /// 音频最大时长（秒）。
    pub max_audio_duration: Option<f64>,
    /// 视频最大时长（秒）。
    pub max_video_duration: Option<f64>,
    /// 超限处理模式。
    pub overflow_mode: OverflowMode,
    /// 自动降级策略配置（仅当 `overflow_mode = Auto` 时生效）。
    pub auto_strategy: AutoStrategy,
}

/// 文件内容超出约束时的处理策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverflowMode {
    /// 直接报错。
    #[default]
    Strict,
    /// 记录警告但继续处理。
    Warn,
    /// 自动降级处理，由 `AutoStrategy` 配置具体策略。
    Auto,
}

/// 自动降级策略配置。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoStrategy {
    /// 图片压缩：目标最大宽度（保持纵横比）。
    pub image_target_width: Option<u32>,
    /// 图片压缩：目标最大高度（保持纵横比）。
    pub image_target_height: Option<u32>,
    /// 图片压缩质量（0-100，仅 JPEG/WebP）。
    /// 构造时须校验此值落在有效范围内，否则压缩行为未定义。
    pub image_quality: u8,
    /// 音频截断：保留前 N 秒。
    pub audio_keep_first_seconds: Option<f64>,
    /// PDF 分块：每份最大页数。
    /// 实际文本提取与 token 化由外部（Agent/LLM 层）执行。
    pub pdf_max_pages_per_chunk: Option<usize>,
    /// 视频降级：提取第 N 帧作为图片替代。
    pub video_extract_frame_at: Option<f64>,
}

impl Default for AutoStrategy {
    fn default() -> Self {
        Self {
            image_target_width: Some(2048),
            image_target_height: Some(2048),
            image_quality: 85,
            audio_keep_first_seconds: None,
            pdf_max_pages_per_chunk: None,
            video_extract_frame_at: Some(0.0),
        }
    }
}
```

### 5.2 路径安全

`FileSource::Local` 在加载时自动执行路径规范化与遍历检测。
`DefaultFileLoader` 使用构造时指定的 `base_dir`（未指定则用 `current_dir()`）作为沙箱边界。
**实现要求**：`base_dir` 在构造时须解析为绝对路径（`canonicalize`），确保 `starts_with` 比较可靠。

```rust
impl DefaultFileLoader {
    fn resolve_local_path(&self, file: &File) -> Result<std::path::PathBuf, LoadError> {
        let base = self.base_dir.as_ref()
            .map(|p| p.canonicalize())
            .unwrap_or_else(|| std::env::current_dir())
            .map_err(|e| LoadError::Io(format!("invalid base dir: {e}")))?;
        let path = match &file.source {
            FileSource::Local { path } => path,
            _ => return Err(LoadError::Io("expected local file source".into())),
        };
        let target = base.join(path);
        let target = target.canonicalize()
            .map_err(|e| LoadError::Io(format!("invalid path: {e}")))?;
        if !target.starts_with(&base) {
            return Err(LoadError::PathTraversal(target.display().to_string()));
        }
        Ok(target)
    }
}
```

**与 toolkit 的一致性**：`FileReadTool` 应将其 `base_dir` 传递给 `DefaultFileLoader::with_base_dir`，
确保工具层和加载层使用同一沙箱边界，避免不一致的安全策略。

> **已知限制**：`canonicalize()` 与实际文件打开之间存在 TOCTOU（Time-of-check to time-of-use）窗口。
> 对于需要绝对路径安全的环境，应在打开文件时使用 `O_NOFOLLOW` 等底层标志，但这超出了本 crate 的范围。

### 5.3 约束校验流程

`FileLoader` 为纯加载职责，**不内置约束校验**。校验由调用方按以下流程显式执行：

1. **用户层约束**：调用方通过 `FileConstraints::check()` 校验。
2. **Provider 层约束**：`ProviderFormat::format_content` 内部校验 `ProviderConstraints`。
3. 根据 `OverflowMode` 处理结果：
   - **`Strict`**：`check()` 返回 `Err` 时直接终止流程。
   - **`Warn`**：`check()` 返回 `Err` 时，将错误信息转为警告追加到 `LoadedContent.warnings`（或单独记录），继续后续处理。
   - **`Auto`**：调用方执行 `AutoStrategy` 降级（如图片压缩、音频截断），然后重新 `check()`，若仍失败则按 `Strict` 处理。

> **注意**：`check()` 返回 `Result<(), ConstraintError>`，只能表达"通过"或"失败"。
> `Warn` 模式下，调用方需自行将 `ConstraintError` 转为警告文本，不依赖 `check()` 内部处理。

```rust
/// 约束校验错误。
#[derive(Error, Debug, Clone)]
pub enum ConstraintError {
    #[error("size {actual} exceeds limit {limit}")]
    SizeExceeded { actual: u64, limit: u64 },
    #[error("media type {0} not in allowed list")]
    TypeNotAllowed(MediaType),
    #[error("image dimensions {width}x{height} exceed limit")]
    ImageDimensionsExceeded { width: u32, height: u32 },
    #[error("duration {actual}s exceeds limit {limit}s")]
    DurationExceeded { actual: f64, limit: f64 },
}

impl FileConstraints {
    /// 校验内容是否满足约束。
    ///
    /// **元数据缺失时的策略**：若 `content` 中某维度元数据为 `None`
    ///（如未启用 `image-meta` 导致 `width`/`height` 未知），该维度约束视为无法验证，
    /// 直接通过（不报错）。若调用方要求严格验证，应先确保对应 feature 已启用。
    pub fn check(&self, content: &MediaContent) -> Result<(), ConstraintError> {
        if let Some(limit) = self.max_size_bytes {
            if content.size_bytes() > limit {
                return Err(ConstraintError::SizeExceeded {
                    actual: content.size_bytes(),
                    limit,
                });
            }
        }
        if let Some(ref allowed) = self.allowed_media_types {
            if !allowed.contains(&content.media_type()) {
                return Err(ConstraintError::TypeNotAllowed(content.media_type()));
            }
        }
        // 图片尺寸校验
        if let MediaContent::Image(img) = content {
            if let (Some(max_w), Some(w)) = (self.max_image_width, img.width) {
                if w > max_w {
                    return Err(ConstraintError::ImageDimensionsExceeded {
                        width: w,
                        height: img.height.unwrap_or(0),
                    });
                }
            }
            if let (Some(max_h), Some(h)) = (self.max_image_height, img.height) {
                if h > max_h {
                    return Err(ConstraintError::ImageDimensionsExceeded {
                        width: img.width.unwrap_or(0),
                        height: h,
                    });
                }
            }
        }

        // 音频时长校验
        if let MediaContent::Audio(audio) = content {
            if let Some(limit) = self.max_audio_duration {
                if let Some(duration) = audio.duration {
                    if duration > limit {
                        return Err(ConstraintError::DurationExceeded { actual: duration, limit });
                    }
                }
            }
        }

        // 视频时长校验
        if let MediaContent::Video(video) = content {
            if let Some(limit) = self.max_video_duration {
                if let Some(duration) = video.duration {
                    if duration > limit {
                        return Err(ConstraintError::DurationExceeded { actual: duration, limit });
                    }
                }
            }
        }

        Ok(())
    }
}
```


---

## 6. 与 pawbun-toolkit 集成

`pawbun-files` 是 `pawbun-toolkit` 的底层依赖。Toolkit 的工具直接消费和产出 `File` / `MediaContent`，实现文件操作与 Agent 工具链的无缝衔接。

### 6.1 工具层集成

> **注意**：以下示例基于 `pawbun-toolkit` 的演进设计（目标 Alpha 阶段）。
> 当前 MVP 版本的 `Tool` trait 返回值可能不同，集成时请参照对应版本的 toolkit API。

```rust
// pawbun-toolkit 的 FileReadTool 使用 pawbun-files 类型
use pawbun_files::{File, FileLoader, DefaultFileLoader, MediaContent};
use pawbun_toolkit::{Tool, ToolResult, ToolError};

#[derive(Debug)]
pub struct FileReadTool {
    loader: DefaultFileLoader,
}

impl FileReadTool {
    pub fn new() -> Self {
        Self { loader: DefaultFileLoader::new() }
    }

    /// 设置沙箱边界，确保 loader 和工具使用同一 base_dir。
    pub fn with_base_dir<P: Into<std::path::PathBuf>>(mut self, base_dir: P) -> Self {
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

    fn execute(&self, input: &str) -> Result<ToolResult, ToolError> {
        // input 为 JSON: {"path": "./report.pdf"}
        let path: String = serde_json::from_str(input)
            .map_err(|e| ToolError::InvalidInput(e.to_string()))?;

        let file = File::from_path(&path);

        // 加载（路径安全由 DefaultFileLoader 统一处理）
        // 注：PathTraversal / TypeMismatch 等输入类错误建议映射为 ToolError::InvalidInput
        let loaded = self.loader.load(&file)
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        // 工具间传递：使用 ProviderFormat 或直接序列化 LoadedContent
        // 这里用 serde_json 序列化 LoadedContent（bytes 字段自动 Base64）
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

### 6.2 多模态 Agent 的文件传递

Agent 通过 `ToolRegistry` 发现 `FileReadTool`，读取文件后获得 `MediaContent`，再通过 `ProviderFormat` 格式化为 LLM 请求：

```rust
use pawbun_files::{File, DefaultFileLoader, OpenAiFormat, ProviderFormat};
use pawbun_toolkit::{ToolKit, ToolExecutor};

/// Agent 加载文件并发送到 OpenAI 的完整流程。
pub fn send_file_to_llm(toolkit: &ToolKit, file_path: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // 1. 通过 toolkit 的 FileReadTool 加载文件
    let result = toolkit.execute("file_read", &format!("{{\"path\":\"{file_path}\"}}"))?;
    // FileReadTool 返回的是序列化后的 MediaContent（bytes 自动 Base64）
    let content: MediaContent = serde_json::from_str(&result.content)?;

    // 2. 使用 ProviderFormat 转换为 OpenAI 格式
    let formatter = OpenAiFormat;
    let message_content = formatter.format_content(&content)?;

    // 3. 构造 OpenAI messages 请求体（由外部 LLM client 发送）
    let request = serde_json::json!({
        "model": "gpt-4o",
        "messages": [{
            "role": "user",
            "content": [message_content]
        }]
    });

    Ok(request)
}
```

### 6.3 跨工具文件传递

`MediaContent` 作为统一格式，可在多个工具间传递：

```
FileReadTool → MediaContent → VisionTool（图片分析）
             → MediaContent → EmbeddingTool（PDF 文本向量化）
             → MediaContent → CodeExecuteTool（代码文件执行）
```

---

## 7. 扩展机制

### 7.1 自定义 FileLoader

实现 `FileLoader` trait 以支持自定义存储后端（如 S3、IPFS、内部 CDN）：

```rust
use pawbun_files::{FileLoader, File, LoadedContent, LoadError};

#[derive(Debug)]
pub struct S3FileLoader {
    bucket: String,
    client: aws_sdk_s3::Client,
}

impl FileLoader for S3FileLoader {
    fn load(&self, file: &File) -> Result<LoadedContent, LoadError> {
        match &file.source {
            FileSource::Url { url } if url.starts_with("s3://") => {
                // 从 S3 下载并解析为 LoadedContent
            }
            _ => Err(LoadError::UnsupportedFormat("S3Loader only supports s3:// URLs".into())),
        }
    }
}
```

### 7.2 自定义 ProviderFormat

为新的 LLM Provider 实现格式化器：

```rust
use pawbun_files::{ProviderFormat, MediaContent, ProviderConstraints};

#[derive(Debug, Clone)]
pub struct CustomProviderFormat;

impl ProviderFormat for CustomProviderFormat {
    fn provider_name(&self) -> &str { "custom_llm" }

    fn format_content(&self, content: &MediaContent) -> Result<serde_json::Value, FormatError> {
        // 自定义序列化逻辑
    }

    fn constraints(&self, media_type: MediaType) -> ProviderConstraints {
        ProviderConstraints {
            max_size_bytes: 10 * 1024 * 1024,
            max_files_per_request: 5,
            supported_mime_types: vec!["image/png".into(), "text/plain".into()],
        }
    }
}
```

### 7.3 自定义 MediaType（未来）

通过 feature flag 或编译期插件扩展新的媒体类型。当前 `#[non_exhaustive]` 属性已预留扩展空间。

### 7.4 测试策略

本 crate 的测试应覆盖以下场景：

#### 单元测试

- **`MediaType::from_extension`**：验证常见扩展名（`.png`, `.jpg`, `.pdf`, `.mp4` 等）的正确推断。
- **`FileConstraints::check`**：边界值测试（恰好等于限制、超过 1 字节、未设置限制等）。
- **`ProviderFormat::format_content`**：使用 `serde_json::Value` 断言输出结构（如 OpenAI 的 `image_url` 字段是否包含正确的 `data:image/png;base64,...` 前缀）。

#### Mock 与自定义 Loader

```rust
use pawbun_files::{
    FileLoader, File, FileSource, FileMetadata, LoadedContent, LoadError,
    MediaContent, TextContent,
};

#[derive(Debug)]
struct MockLoader;

impl FileLoader for MockLoader {
    fn load(&self, file: &File) -> Result<LoadedContent, LoadError> {
        match &file.source {
            FileSource::Bytes { data } => Ok(LoadedContent {
                content: MediaContent::Text(TextContent {
                    text: String::from_utf8_lossy(data).into(),
                    encoding: Some("utf-8".into()),
                }),
                metadata: Default::default(),
                warnings: vec![],
            }),
            _ => Err(LoadError::UnsupportedFormat("mock only supports Bytes".into())),
        }
    }

    fn metadata(&self, _file: &File) -> Result<FileMetadata, LoadError> {
        Ok(FileMetadata::default())
    }
}
```

#### 路径安全测试用例

| 用例 | `base_dir` | 输入路径 | 预期结果 |
|---|---|---|---|
| 正常访问 | `/app/data` | `./report.pdf` | ✅ 通过 |
| 路径遍历 | `/app/data` | `../etc/passwd` | ❌ `PathTraversal` |
| 符号链接突破 | `/app/data` | `link -> /etc/passwd` | ❌ `PathTraversal`（`canonicalize` 后解析为真实路径）|
| 绝对路径注入 | `/app/data` | `/etc/passwd` | ❌ `PathTraversal`（`join` 后仍须落在 `base` 内）|

#### 集成测试

- **`DefaultFileLoader`**：使用 `tempfile` crate 创建临时文件，验证加载、元数据提取、约束校验的端到端流程。
- **Provider 矩阵**：为每个内置 `ProviderFormat` 编写快照测试（snapshot test），确保 JSON 输出结构稳定。

---

## 8. 依赖策略

| 依赖 | 用途 | Feature Flag | 是否必须 |
|---|---|---|---|
| `serde` + `serde_json` | 结构化序列化 / Provider 格式化输出 | — | **是** |
| `thiserror` | 错误类型定义 | — | **是** |
| `base64` | Provider Base64 编码 | — | **是** |
| `bytes`（需启用 `serde` feature） | 共享字节缓冲区（引用计数，避免大文件克隆复制） | — | **是** |
| `reqwest` | URL 来源 HTTP 下载 | `url-source` | 否 |
| `image` | 图片元数据解析（尺寸、格式） | `image-meta` | 否 |
| — | 异步 trait 基于 Rust 1.75+ 原生 `async fn` | — | 否（Alpha 引入，需 MSRV 1.75） |
| `tokio` | 异步运行时（dev-dep） | — | 仅测试 |
| `tracing` | 调用链追踪（可选） | `tracing` | 否（P1） |

### 8.1 Feature 设计

```toml
[features]
default = ["url-source", "image-meta"]
url-source = ["reqwest"]
image-meta = ["image"]
# async 支持基于 Rust 1.75+ 原生 async fn in trait，无需额外依赖
tracing = ["dep:tracing"]
full = ["url-source", "image-meta", "tracing"]
```

### 8.2 零成本抽象保障

- 未启用 `async` 时，所有异步 trait 和运行时相关代码通过条件编译完全消除。
- 未启用 `url-source` 时，`FileSource::Url` 仍可构造，但 `DefaultFileLoader` 对其调用返回 `LoadError::UnsupportedFormat`。
- 未启用 `image-meta` 时，`ImageContent` 的 `width` / `height` 为 `None`，字段本身仍占用内存（两个 `Option<u32>` 约 16 字节），不影响其他功能。

---

## 9. 路线图

| 阶段 | 版本 | 内容 |
|---|---|---|
| **MVP** | 0.1.0 | `File` / `FileSource` / `MediaType` / `MediaContent`（Text + Image + Binary）+ `DefaultFileLoader`（本地文件）+ `OpenAiFormat` |
| **Alpha** | 0.2.0 | `AsyncFileLoader` + `url-source` feature（HTTP 下载）+ `AnthropicFormat` + `FileConstraints`（大小限制） |
| **Beta** | 0.3.0 | `PdfContent` / `AudioContent` / `VideoContent` + `image-meta` feature + 完整 Provider 矩阵（Gemini / Azure）+ `OverflowMode::Auto`（图片压缩） |
| **Stable** | 0.5.0 | API 冻结 + `tracing` 集成 + 性能优化（零拷贝路径）+ 完善文档与示例 |
| **1.0** | 1.0.0 | 生产就绪 + 向后兼容保证 |

### 9.1 各阶段接口演进

| 组件 | MVP | Alpha | Beta | Stable |
|---|---|---|---|---|
| `File` | ✅ | ✅ | ✅ | ✅ |
| `FileSource::Local` | ✅ | ✅ | ✅ | ✅ |
| `FileSource::Url` | ✅（需 feature） | ✅ | ✅ | ✅ |
| `FileSource::Bytes` | ✅ | ✅ | ✅ | ✅ |
| `MediaContent::Text` | ✅ | ✅ | ✅ | ✅ |
| `MediaContent::Image` | ✅ | ✅ | ✅ | ✅ |
| `MediaContent::Pdf` | ❌ | ❌ | ✅ | ✅ |
| `MediaContent::Audio` | ❌ | ❌ | ✅ | ✅ |
| `MediaContent::Video` | ❌ | ❌ | ✅ | ✅ |
| `FileLoader`（同步） | ✅ | ✅ | ✅ | ✅ |
| `AsyncFileLoader` | ❌ | ✅ | ✅ | ✅ |
| `ProviderFormat` | ✅（OpenAI） | ✅（+Anthropic） | ✅（+Gemini/Azure） | ✅ |
| `FileConstraints` | ❌ | ✅（大小限制） | ✅（完整） | ✅ |
| `OverflowMode::Auto` | ❌ | ❌ | ✅ | ✅ |

---

## 10. 附录

### 10.1 与 CrewAI Files 的 API 映射

| CrewAI (Python) | pawbun-files (Rust) | 说明 |
|---|---|---|
| `File(source="...")` | `File::from_path(...)` / `File::from_url(...)` | 统一入口 |
| `ImageFile` / `PDFFile` / ... | `File::from_path(...).with_media_type(MediaType::Image(...))` | 类型通过 `MediaType` 指定 |
| `source=FileBytes(data, filename)` | `File::from_bytes(bytes::Bytes::from(data), "filename.png")` | 内存字节源 |
| `input_files={"key": file}` | `HashMap<String, File>` | Agent/Workflow 传递 |
| Provider 自动格式化 | `ProviderFormat::format_content()` | 显式 trait 调用 |
| `mode="strict"` / `"warn"` / `"auto"` | `OverflowMode::Strict` / `Warn` / `Auto` | 超限处理模式 |
| `file.mode` | `FileConstraints::overflow_mode` | 约束配置 |

### 10.2 从当前骨架代码迁移

当前 `lib.rs`（约 82 行）使用 `FileHandler` trait + 简单 `MediaType` / `MediaContent`。迁移到本 Spec 的新 API 需完成以下步骤：

| 旧 API | 新 API | 说明 |
|---|---|---|
| `MediaType::Image` | `MediaType::Image(ImageFormat::Png)` 等 | 必须指定具体格式；也可用 `matches!(ty, MediaType::Image(_))` 通配匹配 |
| `MediaContent::Binary(bytes)` | `MediaContent::Binary(BinaryContent { bytes, guessed_type: None })` | `Binary` 变体已包裹 `BinaryContent` struct |
| `MediaContent::Text(text)` | `MediaContent::Text(TextContent { text, encoding: None })` | `Text` 变体已包裹 `TextContent` struct |
| `FileHandler` trait | `FileLoader` trait | 输入从 `&Path` 变为 `&File`，输出从 `MediaContent` 变为 `LoadedContent` |
| `FileHandler::read(path)` | `FileLoader::load(file)` | 需通过 `File::from_path(path)` 构造 `File` 后再传入 |
| 自定义 handler 内部 `std::fs::read(path)` | 通过 `match &file.source` 处理 `FileSource::Local` / `Url` / `Bytes` | 或使用 `DefaultFileLoader` 作为底层 |

### 10.3 完整使用示例：多模态 Agent

```rust
use std::collections::HashMap;
use pawbun_files::{
    File, DefaultFileLoader, FileLoader, MediaContent,
    OpenAiFormat, ProviderFormat,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let loader = DefaultFileLoader::new();

    // 构造多模态输入（key 供上层模板系统引用）
    let chart = File::from_path("./sales.png").with_key("sales_chart");
    let report = File::from_path("./report.pdf").with_key("annual_report");

    // 加载内容
    let chart_loaded = loader.load(&chart)?;
    let report_loaded = loader.load(&report)?;

    // 格式化为 OpenAI 请求
    let formatter = OpenAiFormat;
    let chart_block = formatter.format_content(&chart_loaded.content)?;
    let report_block = formatter.format_content(&report_loaded.content)?;

    let request = serde_json::json!({
        "model": "gpt-4o",
        "messages": [{
            "role": "user",
            "content": [
                { "type": "text", "text": "Analyze the sales chart and cross-reference with the PDF report." },
                chart_block,
                report_block,
            ]
        }]
    });

    println!("{}", serde_json::to_string_pretty(&request)?);
    Ok(())
}
```

### 10.4 关键设计收益总结

| 场景 | 若直接传递原始字节 | 使用 pawbun-files |
|---|---|---|
| 多 Provider 切换 | 每个 Agent 手写格式转换 | 换 `ProviderFormat` 实现即可 |
| 文件来源切换 | 本地/URL/字节需不同处理代码 | `FileSource` 屏蔽差异 |
| 文件安全 | 分散在各工具中实现 | `FileLoader` 统一路径校验与约束检查 |
| 测试 | 依赖真实文件系统 / 网络 | `FileSource::Bytes` 可注入模拟数据 |
| 类型安全 | 运行时魔法推断 | `MediaType` + `MediaContent` 编译期匹配 |

---

*End of Specification*
