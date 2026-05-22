//! MCP Server for exposing Pawbun tools via Model Context Protocol.
//!
//! # Quick Start
//!
//! ```no_run
//! use pawbun_mcp_server::McpServer;
//! use pawbun_mcp_core::transport::ServerTransportConfig;
//! use pawbun_toolkit::{ToolKit, FileReadTool};
//! use pawbun_files::DefaultFileLoader;
//!
//! let mut toolkit = ToolKit::new();
//! toolkit.register(Box::new(FileReadTool::default()));
//!
//! let loader = DefaultFileLoader::with_base_dir("/app/data");
//!
//! let server = McpServer::builder("pawbun", "0.1.0")
//!     .register_toolkit(toolkit)
//!     .register_file_loader(loader)
//!     .build();
//!
//! // Blocking stdio server
//! server.launch(ServerTransportConfig::Stdio).unwrap();
//! ```

pub mod error;
pub mod handler;
pub mod server;
pub mod tool_bridge;
pub mod transport;

pub use error::McpServerError;
pub use server::{McpServer, McpServerBuilder};
