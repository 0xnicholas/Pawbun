//! McpServer and McpServerBuilder — implemented in Task 2.4.

use pawbun_mcp_core::protocol::ServerInfo;
use pawbun_mcp_core::transport::{ServerTransport, ServerTransportConfig};
use pawbun_toolkit::{Tool, ToolKit};
use pawbun_files::DefaultFileLoader;
use serde_json::{json, Value};

use crate::error::McpServerError;
use crate::handler::RequestHandler;

/// MCP Server exposing Pawbun tools via Model Context Protocol.
pub struct McpServer {
    toolkit: ToolKit,
    server_info: ServerInfo,
    capabilities: Value,
}

/// Builder for [`McpServer`].
pub struct McpServerBuilder {
    toolkit: ToolKit,
    file_loader: Option<DefaultFileLoader>,
    server_name: String,
    server_version: String,
    capabilities: Value,
}

impl McpServerBuilder {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            toolkit: ToolKit::new(),
            file_loader: None,
            server_name: name.into(),
            server_version: version.into(),
            capabilities: json!({"tools": {}}),
        }
    }

    /// Register a ToolKit. Assigns directly (single-toolkit).
    pub fn register_toolkit(mut self, toolkit: ToolKit) -> Self {
        self.toolkit = toolkit;
        self
    }

    /// Register a FileLoader. Automatically wraps as bridge tools on build.
    ///
    /// **Deduplication**: user-registered tools with the same name take priority.
    pub fn register_file_loader(mut self, loader: DefaultFileLoader) -> Self {
        self.file_loader = Some(loader);
        self
    }

    /// Register a single tool. Same-name overwrites.
    pub fn register_tool(mut self, tool: Box<dyn Tool>) -> Self {
        self.toolkit.register(tool);
        self
    }

    /// Override default capabilities (default: `{"tools": {}}`).
    pub fn capabilities(mut self, caps: Value) -> Self {
        self.capabilities = caps;
        self
    }

    /// Build the [`McpServer`], registering all bridge tools.
    pub fn build(mut self) -> McpServer {
        if let Some(loader) = self.file_loader.take() {
            crate::tool_bridge::register_bridge_tools(&mut self.toolkit, loader);
        }
        McpServer {
            toolkit: self.toolkit,
            server_info: ServerInfo {
                name: self.server_name,
                version: self.server_version,
            },
            capabilities: self.capabilities,
        }
    }
}

impl McpServer {
    pub fn builder(name: impl Into<String>, version: impl Into<String>) -> McpServerBuilder {
        McpServerBuilder::new(name, version)
    }

    pub fn launch(self, config: ServerTransportConfig) -> Result<(), McpServerError> {
        // Stub: implemented in Task 2.4
        Err(McpServerError::Bind("not yet implemented".into()))
    }
}
