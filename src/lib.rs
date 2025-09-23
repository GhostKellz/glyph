pub mod protocol;
pub mod transport;
pub mod server;
pub mod client;

pub use protocol::*;
pub use transport::*;

// Re-export commonly used types for convenience
pub use protocol::{
    GlyphError, Result, JsonRpcMessage, JsonRpcRequest, JsonRpcResponse, JsonRpcNotification,
    McpError, RequestId, Implementation, Content, Tool, Resource, Prompt,
    InitializeRequest, InitializeResult, CallToolRequest, CallToolResult,
    ListToolsRequest, ListToolsResult, ReadResourceRequest, ReadResourceResult,
    ProtocolVersion, ClientCapabilities, ServerCapabilities,
};

pub use transport::{
    Transport, TransportServer, TransportConfig, TransportType,
    StdioTransport, WebSocketTransport, WebSocketServer, HttpTransport,
};

// Convenience re-exports
pub mod json {
    pub use serde_json::*;
}

// Re-export async_trait for convenience
pub use async_trait::async_trait;

// Common result type alias
pub type McpResult<T> = std::result::Result<T, McpError>;

// Library version and info
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = env!("CARGO_PKG_NAME");

pub fn library_info() -> Implementation {
    Implementation::new(NAME, VERSION)
}