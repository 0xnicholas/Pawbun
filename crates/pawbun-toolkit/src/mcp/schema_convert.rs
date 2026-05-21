//! Convert MCP input_schema to [`ToolParameter`] list.

use serde_json::Value;

use crate::ToolParameter;

/// Converts an MCP `input_schema` (JSON Schema object) into a list of [`ToolParameter`].
///
/// Supports simple object schemas with `properties` and `required` fields.
/// Properties schemas are stored as-is in [`ToolParameter::schema`].
///
/// # Example
/// ```
/// use pawbun_toolkit::mcp::schema_convert::schema_to_parameters;
/// use serde_json::json;
///
/// let schema = json!({
///     "type": "object",
///     "properties": {
///         "path": {"type": "string", "description": "File path"},
///         "recursive": {"type": "boolean"}
///     },
///     "required": ["path"]
/// });
///
/// let params = schema_to_parameters(&schema);
/// assert_eq!(params.len(), 2);
/// assert_eq!(params[0].name, "path");
/// assert!(params[0].required);
/// assert!(!params[1].required);
/// ```
pub fn schema_to_parameters(schema: &Value) -> Vec<ToolParameter> {
    let mut params = Vec::new();

    let Some(properties) = schema.get("properties").and_then(|p| p.as_object()) else {
        return params;
    };

    let required: Vec<&str> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    for (name, prop_schema) in properties {
        let description = prop_schema
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();
        let is_required = required.contains(&name.as_str());

        params.push(ToolParameter {
            name: name.clone(),
            description,
            required: is_required,
            schema: prop_schema.clone(),
        });
    }

    params
}
