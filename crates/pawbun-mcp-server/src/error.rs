use pawbun_mcp_core::transport::TransportError;
use pawbun_toolkit::ToolError;
use pawbun_files::LoadError;

/// Errors that can occur in MCP Server operations.
#[derive(thiserror::Error, Debug)]
pub enum McpServerError {
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),

    #[error("tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("load error: {0}")]
    Load(#[from] LoadError),

    #[error("bind failed: {0}")]
    Bind(String),

    #[error("MCP protocol error: {message} (code {code})")]
    Protocol { message: String, code: i32 },
}
