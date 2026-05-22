use serde::{Deserialize, Serialize};
use serde_json::Value;

// Re-export ToolParameter from pawbun-mcp-core.
pub use pawbun_mcp_core::ToolParameter;

/// 统一工具执行结果。
///
/// 所有工具无论同步或异步，均返回此结构体。`success` 字段明确区分成功与失败，
/// 便于编排器进行错误处理决策。
///
/// # Example
/// ```
/// use pawbun_toolkit::ToolResult;
///
/// let result = ToolResult {
///     success: true,
///     content: "Hello, world!".into(),
///     metadata: None,
///     elapsed_ms: Some(42),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// 执行是否成功。
    pub success: bool,
    /// 执行返回的文本内容。
    pub content: String,
    /// 附加元数据（如 HTTP 状态码、文件大小等）。
    pub metadata: Option<Value>,
    /// 执行耗时（毫秒），由调用方或拦截层填充。
    pub elapsed_ms: Option<u64>,
}
