use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, thiserror::Error)]
pub struct McpError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ErrorCode {
    Standard(StandardErrorCode),
    Custom(i32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StandardErrorCode {
    #[serde(rename = "-32700")]
    ParseError,
    #[serde(rename = "-32600")]
    InvalidRequest,
    #[serde(rename = "-32601")]
    MethodNotFound,
    #[serde(rename = "-32602")]
    InvalidParams,
    #[serde(rename = "-32603")]
    InternalError,
    #[serde(rename = "-32000")]
    ToolNotFound,
    #[serde(rename = "-32001")]
    ToolExecutionError,
    #[serde(rename = "-32002")]
    ResourceNotFound,
    #[serde(rename = "-32003")]
    ResourceAccessDenied,
    #[serde(rename = "-32004")]
    PromptNotFound,
    #[serde(rename = "-32005")]
    PromptExecutionError,
    #[serde(rename = "-32006")]
    ConsentRequired,
    #[serde(rename = "-32007")]
    AuthenticationRequired,
    #[serde(rename = "-32008")]
    RateLimitExceeded,
    #[serde(rename = "-32009")]
    ServerOverloaded,
    #[serde(rename = "-32010")]
    ProtocolVersionMismatch,
}

impl From<StandardErrorCode> for ErrorCode {
    fn from(code: StandardErrorCode) -> Self {
        ErrorCode::Standard(code)
    }
}

impl From<i32> for ErrorCode {
    fn from(code: i32) -> Self {
        ErrorCode::Custom(code)
    }
}

impl McpError {
    pub fn new(code: impl Into<ErrorCode>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn parse_error(message: impl Into<String>) -> Self {
        Self::new(StandardErrorCode::ParseError, message)
    }

    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(StandardErrorCode::InvalidRequest, message)
    }

    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self::new(
            StandardErrorCode::MethodNotFound,
            format!("Method not found: {}", method.into()),
        )
    }

    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(StandardErrorCode::InvalidParams, message)
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::new(StandardErrorCode::InternalError, message)
    }

    pub fn tool_not_found(tool_name: impl Into<String>) -> Self {
        Self::new(
            StandardErrorCode::ToolNotFound,
            format!("Tool not found: {}", tool_name.into()),
        )
    }

    pub fn tool_execution_error(message: impl Into<String>) -> Self {
        Self::new(StandardErrorCode::ToolExecutionError, message)
    }

    pub fn resource_not_found(uri: impl Into<String>) -> Self {
        Self::new(
            StandardErrorCode::ResourceNotFound,
            format!("Resource not found: {}", uri.into()),
        )
    }

    pub fn consent_required(message: impl Into<String>) -> Self {
        Self::new(StandardErrorCode::ConsentRequired, message)
    }
}

impl fmt::Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorCode::Standard(code) => write!(f, "{}", code.value()),
            ErrorCode::Custom(code) => write!(f, "{}", code),
        }
    }
}

impl StandardErrorCode {
    pub fn value(&self) -> i32 {
        match self {
            StandardErrorCode::ParseError => -32700,
            StandardErrorCode::InvalidRequest => -32600,
            StandardErrorCode::MethodNotFound => -32601,
            StandardErrorCode::InvalidParams => -32602,
            StandardErrorCode::InternalError => -32603,
            StandardErrorCode::ToolNotFound => -32000,
            StandardErrorCode::ToolExecutionError => -32001,
            StandardErrorCode::ResourceNotFound => -32002,
            StandardErrorCode::ResourceAccessDenied => -32003,
            StandardErrorCode::PromptNotFound => -32004,
            StandardErrorCode::PromptExecutionError => -32005,
            StandardErrorCode::ConsentRequired => -32006,
            StandardErrorCode::AuthenticationRequired => -32007,
            StandardErrorCode::RateLimitExceeded => -32008,
            StandardErrorCode::ServerOverloaded => -32009,
            StandardErrorCode::ProtocolVersionMismatch => -32010,
        }
    }
}

#[derive(Debug, Error)]
pub enum GlyphError {
    #[error("MCP error: {0}")]
    Mcp(#[from] McpError),
    #[error("JSON-RPC error: {0}")]
    JsonRpc(String),
    #[error("Transport error: {0}")]
    Transport(String),
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Protocol version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },
    #[error("Connection closed")]
    ConnectionClosed,
    #[error("Timeout")]
    Timeout,
}

impl From<crate::Error> for GlyphError {
    fn from(err: crate::Error) -> Self {
        match err {
            crate::Error::JsonRpc(msg) => GlyphError::JsonRpc(msg),
            crate::Error::Protocol(msg) => GlyphError::JsonRpc(msg),
            crate::Error::Transport(msg) => GlyphError::Transport(msg),
            crate::Error::Auth(msg) => GlyphError::JsonRpc(format!("Authentication failed: {}", msg)),
            crate::Error::Permission(msg) => GlyphError::JsonRpc(format!("Permission denied: {}", msg)),
            crate::Error::ToolNotFound { name } => GlyphError::Mcp(McpError::tool_not_found(&name)),
            crate::Error::ToolExecution(msg) => GlyphError::Mcp(McpError::tool_execution_error(msg)),
            crate::Error::ResourceNotFound { uri } => GlyphError::Mcp(McpError::resource_not_found(&uri)),
            crate::Error::InvalidRequest(msg) => GlyphError::Mcp(McpError::invalid_request(msg)),
            crate::Error::Connection(msg) => GlyphError::Transport(msg),
            crate::Error::Serialization(err) => GlyphError::Serialization(err),
            crate::Error::Io(err) => GlyphError::Io(err),
            crate::Error::WebSocket(msg) => GlyphError::Transport(msg),
            crate::Error::Http(msg) => GlyphError::Transport(msg),
            crate::Error::ConnectionClosed => GlyphError::ConnectionClosed,
            crate::Error::Timeout(_) => GlyphError::Timeout,
            crate::Error::Internal(msg) => GlyphError::Mcp(McpError::internal_error(msg)),
        }
    }
}

impl From<crate::Error> for McpError {
    fn from(err: crate::Error) -> Self {
        match err {
            crate::Error::JsonRpc(msg) => McpError::invalid_request(msg),
            crate::Error::Protocol(msg) => McpError::internal_error(msg),
            crate::Error::Transport(msg) => McpError::internal_error(format!("Transport error: {}", msg)),
            crate::Error::Auth(msg) => McpError::consent_required(format!("Authentication failed: {}", msg)),
            crate::Error::Permission(msg) => McpError::consent_required(format!("Permission denied: {}", msg)),
            crate::Error::ToolNotFound { name } => McpError::tool_not_found(&name),
            crate::Error::ToolExecution(msg) => McpError::tool_execution_error(msg),
            crate::Error::ResourceNotFound { uri } => McpError::resource_not_found(&uri),
            crate::Error::InvalidRequest(msg) => McpError::invalid_request(msg),
            crate::Error::Connection(msg) => McpError::internal_error(format!("Connection error: {}", msg)),
            crate::Error::Serialization(err) => McpError::internal_error(format!("Serialization error: {}", err)),
            crate::Error::Io(err) => McpError::internal_error(format!("IO error: {}", err)),
            crate::Error::WebSocket(msg) => McpError::internal_error(format!("WebSocket error: {}", msg)),
            crate::Error::Http(msg) => McpError::internal_error(format!("HTTP error: {}", msg)),
            crate::Error::ConnectionClosed => McpError::internal_error("Connection closed".to_string()),
            crate::Error::Timeout(msg) => McpError::internal_error(format!("Timeout: {}", msg)),
            crate::Error::Internal(msg) => McpError::internal_error(msg),
        }
    }
}