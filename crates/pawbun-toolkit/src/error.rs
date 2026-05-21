use thiserror::Error;

/// 工具执行过程中可能发生的错误。
///
/// 所有错误变体均实现 [`Clone`]，便于在需要复制结果的场景（如缓存、重试）中使用。
///
/// # Example
/// ```
/// use pawbun_toolkit::ToolError;
///
/// let err = ToolError::NotFound("unknown_tool".into());
/// assert_eq!(err.to_string(), "tool not found: unknown_tool");
/// ```
#[derive(Error, Debug, Clone)]
pub enum ToolError {
    /// 输入参数无效或格式错误。
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// 工具执行过程中发生不可恢复的错误。
    #[error("execution failed: {0}")]
    ExecutionFailed(String),

    /// 请求的工具未在注册表中找到。
    #[error("tool not found: {0}")]
    NotFound(String),

    /// 工具执行超过指定超时时间。
    #[error("timeout after {0}ms")]
    Timeout(u64),

    /// 输入输出序列化或反序列化失败。
    #[error("serialization error: {0}")]
    Serialization(String),

    /// IO error (file system, network, etc.).
    #[error("IO error: {message} (kind: {kind:?})")]
    Io {
        message: String,
        kind: std::io::ErrorKind,
    },
}
