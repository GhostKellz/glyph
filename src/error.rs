use std::fmt;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Transport error: {0}")]
    Transport(String),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Permission denied: {0}")]
    Permission(String),

    #[error("Tool not found: {name}")]
    ToolNotFound { name: String },

    #[error("Tool execution failed: {0}")]
    ToolExecution(String),

    #[error("Resource not found: {uri}")]
    ResourceNotFound { uri: String },

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

impl Error {
    pub fn json_rpc<T: fmt::Display>(msg: T) -> Self {
        Self::JsonRpc(msg.to_string())
    }

    pub fn protocol<T: fmt::Display>(msg: T) -> Self {
        Self::Protocol(msg.to_string())
    }

    pub fn transport<T: fmt::Display>(msg: T) -> Self {
        Self::Transport(msg.to_string())
    }

    pub fn connection<T: fmt::Display>(msg: T) -> Self {
        Self::Connection(msg.to_string())
    }

    pub fn websocket<T: fmt::Display>(msg: T) -> Self {
        Self::WebSocket(msg.to_string())
    }

    pub fn http<T: fmt::Display>(msg: T) -> Self {
        Self::Http(msg.to_string())
    }

    pub fn internal<T: fmt::Display>(msg: T) -> Self {
        Self::Internal(msg.to_string())
    }
}

impl From<crate::protocol::error::GlyphError> for Error {
    fn from(err: crate::protocol::error::GlyphError) -> Self {
        match err {
            crate::protocol::error::GlyphError::Mcp(mcp_err) => {
                match mcp_err.code {
                    crate::protocol::error::ErrorCode::Standard(code) => match code {
                        crate::protocol::error::StandardErrorCode::ToolNotFound => {
                            Error::ToolNotFound { name: mcp_err.message }
                        }
                        crate::protocol::error::StandardErrorCode::ToolExecutionError => {
                            Error::ToolExecution(mcp_err.message)
                        }
                        crate::protocol::error::StandardErrorCode::ResourceNotFound => {
                            Error::ResourceNotFound { uri: mcp_err.message }
                        }
                        _ => Error::Protocol(mcp_err.message),
                    },
                    _ => Error::Protocol(mcp_err.message),
                }
            }
            crate::protocol::error::GlyphError::JsonRpc(msg) => Error::JsonRpc(msg),
            crate::protocol::error::GlyphError::Transport(msg) => Error::Transport(msg),
            crate::protocol::error::GlyphError::Serialization(err) => Error::Serialization(err),
            crate::protocol::error::GlyphError::Io(err) => Error::Io(err),
            crate::protocol::error::GlyphError::VersionMismatch { .. } => Error::Protocol(err.to_string()),
            crate::protocol::error::GlyphError::ConnectionClosed => Error::ConnectionClosed,
            crate::protocol::error::GlyphError::Timeout => Error::Timeout("Timeout".to_string()),
        }
    }
}

impl From<crate::protocol::error::McpError> for Error {
    fn from(err: crate::protocol::error::McpError) -> Self {
        crate::protocol::error::GlyphError::from(err).into()
    }
}