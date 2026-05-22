//! MCP protocol core types and transport abstractions.
//!
//! This crate provides the foundational types shared by both MCP clients
//! and servers in the Pawbun ecosystem.

/// Describes a tool's input parameter for JSON Schema generation.
///
/// This is the canonical definition of `ToolParameter` used by both
/// `pawbun-toolkit` and `pawbun-mcp-server`. The toolkit crate re-exports
/// this type for backward compatibility.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolParameter {
    /// Parameter name.
    pub name: String,
    /// Human-readable description for LLM consumption.
    pub description: String,
    /// Whether this parameter is required.
    pub required: bool,
    /// JSON Schema fragment describing the parameter type.
    pub schema: serde_json::Value,
}

#[cfg(feature = "schemars")]
impl ToolParameter {
    /// Generates a ToolParameter from a type implementing [`schemars::JsonSchema`].
    ///
    /// Requires the `schemars` feature.
    ///
    /// # Example
    /// ```
    /// use pawbun_mcp_core::ToolParameter;
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
        let schema =
            serde_json::to_value(root.schema).expect("schema serialization should not fail");
        Self {
            name: name.into(),
            description: description.into(),
            required,
            schema,
        }
    }
}
