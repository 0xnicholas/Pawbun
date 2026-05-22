//! MCP (Model Context Protocol) adapter layer.
//!
//! Provides connectivity to MCP servers via stdio or SSE transport,
//! and exposes remote MCP tools as local `Tool` trait implementations.
//!
//! Protocol types and transport traits are defined in `pawbun-mcp-core`.
//! This module provides the client implementations and re-exports core types.
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
pub mod transport;

// Re-export from pawbun-mcp-core (backward compatible)
pub use pawbun_mcp_core::protocol::*;
pub use pawbun_mcp_core::schema_convert::*;
pub use pawbun_mcp_core::transport::{
    ServerTransportConfig, Transport, TransportConfig, TransportError,
};

pub use adapter::{McpAdapter, McpError, McpSession};
pub use dynamic_tool::DynamicTool;
pub use transport::{StdioTransport, SseTransport};
