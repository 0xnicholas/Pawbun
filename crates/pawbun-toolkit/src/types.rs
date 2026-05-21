use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 工具的输入参数描述（JSON Schema 子集）。
///
/// 每个参数通过 JSON Schema fragment 描述其类型和约束，便于 Agent 构造合法输入。
///
/// # Example
/// ```
/// use pawbun_toolkit::ToolParameter;
/// use serde_json::json;
///
/// let param = ToolParameter {
///     name: "path".into(),
///     description: "File path to read".into(),
///     required: true,
///     schema: json!({"type": "string"}),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    /// 参数名。
    pub name: String,
    /// 参数功能描述（供 Agent 理解）。
    pub description: String,
    /// 是否为必填参数。
    pub required: bool,
    /// JSON Schema fragment 描述参数类型。
    pub schema: Value,
}

#[cfg(feature = "schemars")]
impl ToolParameter {
    /// 从实现了 [`schemars::JsonSchema`] 的类型自动生成参数 schema。
    ///
    /// 需要启用 `schemars` feature。
    ///
    /// # Example
    /// ```
    /// use pawbun_toolkit::ToolParameter;
    /// use schemars::JsonSchema;
    /// use serde::Deserialize;
    ///
    /// #[derive(Deserialize, JsonSchema)]
    /// struct MyParams {
    ///     path: String,
    /// }
    ///
    /// let param = ToolParameter::from_schema::<MyParams>("input", "Tool input", true);
    /// assert_eq!(param.name, "input");
    /// ```
    pub fn from_schema<T: schemars::JsonSchema>(
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        let root = schemars::schema_for!(T);
        let schema = serde_json::to_value(root.schema)
            .unwrap_or_else(|e| panic!("schema serialization should not fail: {e}"));
        Self {
            name: name.into(),
            description: description.into(),
            required,
            schema,
        }
    }
}

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
