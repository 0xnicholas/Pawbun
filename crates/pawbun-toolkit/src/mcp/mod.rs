//! MCP (Model Context Protocol) adapter layer.
//!
//! Provides connectivity to MCP servers via stdio or SSE transport,
//! and exposes remote MCP tools as local `Tool` trait implementations.
//!
//! # Example
//! ```no_run
//! use pawbun_toolkit::mcp::{McpAdapter, TransportConfig};
//!
//! let config = TransportConfig::Stdio {
//!     command: "npx".into(),
//!     args: vec!["-y".into(), "@modelcontextprotocol/server-filesystem".into(), "/tmp".into()],
//! };
//! let mut session = McpAdapter::connect(config).unwrap();
//! let tools = session.list_tools().unwrap();
//! ```

pub mod adapter;
pub mod dynamic_tool;
pub mod protocol;
pub mod schema_convert;
pub mod transport;

pub use adapter::{McpAdapter, McpError, McpSession};
pub use dynamic_tool::DynamicTool;
pub use schema_convert::schema_to_parameters;
pub use transport::{StdioTransport, TransportConfig};
