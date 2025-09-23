use serde::{Deserialize, Serialize};
use crate::protocol::{
    ClientCapabilities, Content, Implementation, Prompt, PromptArgument, PromptMessage,
    Resource, ResourceTemplate, ServerCapabilities, Tool, ProtocolVersion,
};
use std::collections::HashMap;

// Initialize Messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeRequest {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: ProtocolVersion,
    pub capabilities: ClientCapabilities,
    #[serde(rename = "clientInfo")]
    pub client_info: Implementation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResult {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: ProtocolVersion,
    pub capabilities: ServerCapabilities,
    #[serde(rename = "serverInfo")]
    pub server_info: Implementation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

// Ping Message (for keep-alive)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingRequest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResult;

// Tool Messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsResult {
    pub tools: Vec<Tool>,
    #[serde(rename = "nextCursor")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResult {
    pub content: Vec<Content>,
    #[serde(rename = "isError")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

impl CallToolResult {
    pub fn success(content: Vec<Content>) -> Self {
        Self {
            content,
            is_error: None,
            meta: None,
        }
    }

    pub fn error(content: Vec<Content>) -> Self {
        Self {
            content,
            is_error: Some(true),
            meta: None,
        }
    }

    pub fn with_meta(mut self, meta: serde_json::Value) -> Self {
        self.meta = Some(meta);
        self
    }
}

// Resource Messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourcesRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourcesResult {
    pub resources: Vec<Resource>,
    #[serde(rename = "nextCursor")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourceTemplatesRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourceTemplatesResult {
    #[serde(rename = "resourceTemplates")]
    pub resource_templates: Vec<ResourceTemplate>,
    #[serde(rename = "nextCursor")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceRequest {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceResult {
    pub contents: Vec<ResourceContents>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResourceContents {
    Text {
        uri: String,
        #[serde(rename = "mimeType")]
        mime_type: Option<String>,
        text: String,
    },
    Blob {
        uri: String,
        #[serde(rename = "mimeType")]
        mime_type: Option<String>,
        blob: String, // base64 encoded
    },
}

impl ResourceContents {
    pub fn text(uri: impl Into<String>, text: impl Into<String>) -> Self {
        Self::Text {
            uri: uri.into(),
            mime_type: None,
            text: text.into(),
        }
    }

    pub fn text_with_mime_type(
        uri: impl Into<String>,
        text: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self::Text {
            uri: uri.into(),
            mime_type: Some(mime_type.into()),
            text: text.into(),
        }
    }

    pub fn blob(uri: impl Into<String>, blob: impl Into<String>) -> Self {
        Self::Blob {
            uri: uri.into(),
            mime_type: None,
            blob: blob.into(),
        }
    }

    pub fn blob_with_mime_type(
        uri: impl Into<String>,
        blob: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self::Blob {
            uri: uri.into(),
            mime_type: Some(mime_type.into()),
            blob: blob.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeRequest {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscribeResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeRequest {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsubscribeResult;

// Prompt Messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPromptsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPromptsResult {
    pub prompts: Vec<Prompt>,
    #[serde(rename = "nextCursor")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPromptRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetPromptResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub messages: Vec<PromptMessage>,
}

// Completion Messages (for LLM integration)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteRequest {
    pub argument: CompletionArgument,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionArgument {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompleteResult {
    pub completion: Completion,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Completion {
    pub values: Vec<String>,
    pub total: Option<i32>,
    #[serde(rename = "hasMore")]
    pub has_more: Option<bool>,
}

// Logging Messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingMessageNotification {
    pub level: LogLevel,
    pub data: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logger: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

// Progress Messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressNotification {
    #[serde(rename = "progressToken")]
    pub progress_token: serde_json::Value,
    pub progress: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
}

// Root list changed notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsListChangedNotification;

// Cancellation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelledNotification {
    #[serde(rename = "requestId")]
    pub request_id: crate::protocol::RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

// All message types enumeration for easier handling
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum McpRequest {
    #[serde(rename = "initialize")]
    Initialize(InitializeRequest),
    #[serde(rename = "ping")]
    Ping(PingRequest),
    #[serde(rename = "tools/list")]
    ListTools(ListToolsRequest),
    #[serde(rename = "tools/call")]
    CallTool(CallToolRequest),
    #[serde(rename = "resources/list")]
    ListResources(ListResourcesRequest),
    #[serde(rename = "resources/templates/list")]
    ListResourceTemplates(ListResourceTemplatesRequest),
    #[serde(rename = "resources/read")]
    ReadResource(ReadResourceRequest),
    #[serde(rename = "resources/subscribe")]
    Subscribe(SubscribeRequest),
    #[serde(rename = "resources/unsubscribe")]
    Unsubscribe(UnsubscribeRequest),
    #[serde(rename = "prompts/list")]
    ListPrompts(ListPromptsRequest),
    #[serde(rename = "prompts/get")]
    GetPrompt(GetPromptRequest),
    #[serde(rename = "completion/complete")]
    Complete(CompleteRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpResult {
    Initialize(InitializeResult),
    Ping(PingResult),
    ListTools(ListToolsResult),
    CallTool(CallToolResult),
    ListResources(ListResourcesResult),
    ListResourceTemplates(ListResourceTemplatesResult),
    ReadResource(ReadResourceResult),
    Subscribe(SubscribeResult),
    Unsubscribe(UnsubscribeResult),
    ListPrompts(ListPromptsResult),
    GetPrompt(GetPromptResult),
    Complete(CompleteResult),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum McpNotification {
    #[serde(rename = "notifications/message")]
    LoggingMessage(LoggingMessageNotification),
    #[serde(rename = "notifications/progress")]
    Progress(ProgressNotification),
    #[serde(rename = "notifications/roots/list_changed")]
    RootsListChanged(RootsListChangedNotification),
    #[serde(rename = "notifications/cancelled")]
    Cancelled(CancelledNotification),
    #[serde(rename = "notifications/tools/list_changed")]
    ToolListChanged,
    #[serde(rename = "notifications/resources/list_changed")]
    ResourceListChanged,
    #[serde(rename = "notifications/prompts/list_changed")]
    PromptListChanged,
}