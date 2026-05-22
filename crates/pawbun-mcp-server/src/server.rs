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
///
/// # Example
/// ```no_run
/// use pawbun_mcp_server::McpServer;
/// use pawbun_mcp_core::transport::ServerTransportConfig;
/// use pawbun_toolkit::{ToolKit, FileReadTool};
/// use pawbun_files::DefaultFileLoader;
///
/// let mut toolkit = ToolKit::new();
/// toolkit.register(Box::new(FileReadTool::default()));
///
/// let loader = DefaultFileLoader::with_base_dir("/app/data");
///
/// let server = McpServer::builder("pawbun", "0.1.0")
///     .register_toolkit(toolkit)
///     .register_file_loader(loader)
///     .build();
///
/// // Blocking stdio server
/// server.launch(ServerTransportConfig::Stdio).unwrap();
/// ```
pub struct McpServerBuilder {
    toolkit: ToolKit,
    file_loader: Option<DefaultFileLoader>,
    server_name: String,
    server_version: String,
    capabilities: Value,
}

impl McpServerBuilder {
    /// Create a builder with server name and version.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            toolkit: ToolKit::new(),
            file_loader: None,
            server_name: name.into(),
            server_version: version.into(),
            capabilities: json!({"tools": {}}),
        }
    }

    /// Register a ToolKit. Assigns directly (single-toolkit for now).
    pub fn register_toolkit(mut self, toolkit: ToolKit) -> Self {
        self.toolkit = toolkit;
        self
    }

    /// Register a FileLoader. Automatically wraps as `file_read` and `file_list` tools.
    ///
    /// **Deduplication**: if a tool with the same name already exists in the toolkit,
    /// the bridge tool is skipped — user-registered tools take priority.
    pub fn register_file_loader(mut self, loader: DefaultFileLoader) -> Self {
        self.file_loader = Some(loader);
        self
    }

    /// Register a single tool. Same-name tools are overwritten.
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
    /// Create a builder.
    pub fn builder(name: impl Into<String>, version: impl Into<String>) -> McpServerBuilder {
        McpServerBuilder::new(name, version)
    }

    /// Start the server with the given transport configuration.
    /// Blocks the current thread until the transport closes.
    pub fn launch(self, config: ServerTransportConfig) -> Result<(), McpServerError> {
        match config {
            ServerTransportConfig::Stdio => {
                let transport =
                    Box::new(crate::transport::stdio::StdioServerTransport::new());
                self.run_loop(transport)
            }
            #[cfg(feature = "http")]
            ServerTransportConfig::Sse { bind_addr } => {
                let transport = crate::transport::sse::SseServerTransport::new(&bind_addr)
                    .map_err(|e| McpServerError::Bind(e))?;
                self.run_loop(Box::new(transport))
            }
            #[cfg(not(feature = "http"))]
            ServerTransportConfig::Sse { .. } => Err(McpServerError::Bind(
                "SSE transport requires the 'http' feature".into(),
            )),
        }
    }

    fn run_loop(
        mut self,
        mut transport: Box<dyn ServerTransport>,
    ) -> Result<(), McpServerError> {
        let mut handler = RequestHandler::new(self.toolkit, self.server_info, self.capabilities);

        loop {
            let req = match transport.recv() {
                Ok(req) => req,
                Err(pawbun_mcp_core::transport::TransportError::UnexpectedEof) => break,
                Err(e) => return Err(e.into()),
            };

            let resp = handler.handle(req);
            transport.send(resp)?;
        }

        transport.close()?;
        Ok(())
    }
}
