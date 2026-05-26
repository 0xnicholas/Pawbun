//! Schema bidirectional conversion example.
//!
//! Demonstrates converting between JSON Schema and ToolParameter lists.

use pawbun_mcp_core::schema_convert::{parameters_to_schema, schema_to_parameters};
use pawbun_mcp_core::ToolParameter;
use serde_json::json;

fn main() {
    // JSON Schema -> ToolParameter list
    let schema = json!({
        "type": "object",
        "properties": {
            "path": {"type": "string", "description": "File path"},
            "recursive": {"type": "boolean"}
        },
        "required": ["path"]
    });

    let params = schema_to_parameters(&schema);
    println!("Schema -> Parameters: {:?}", params);

    // ToolParameter list -> JSON Schema
    let params = vec![
        ToolParameter {
            name: "url".into(),
            description: "URL to fetch".into(),
            required: true,
            schema: json!({"type": "string", "format": "uri"}),
        },
        ToolParameter {
            name: "max_length".into(),
            description: "Max length".into(),
            required: false,
            schema: json!({"type": "integer"}),
        },
    ];

    let schema = parameters_to_schema(&params);
    println!("Parameters -> Schema: {}", serde_json::to_string_pretty(&schema).unwrap());
}
