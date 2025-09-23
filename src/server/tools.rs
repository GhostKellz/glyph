use crate::protocol::{
    Content, Tool as ToolDefinition, ToolInputSchema, CallToolRequest, CallToolResult,
    McpError, RequestId,
};
use crate::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use serde_json::Value;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> Option<&str> {
        None
    }
    fn input_schema(&self) -> ToolInputSchema;

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult>;
}

pub struct ToolContext {
    pub request_id: Option<RequestId>,
    pub client_info: Option<crate::protocol::Implementation>,
    pub metadata: HashMap<String, Value>,
}

impl ToolContext {
    pub fn new() -> Self {
        Self {
            request_id: None,
            client_info: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_request_id(mut self, id: RequestId) -> Self {
        self.request_id = Some(id);
        self
    }

    pub fn with_client_info(mut self, info: crate::protocol::Implementation) -> Self {
        self.client_info = Some(info);
        self
    }

    pub fn with_metadata(mut self, key: String, value: Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

impl Default for ToolContext {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    pub async fn register(&mut self, tool: Box<dyn Tool>) -> Result<()> {
        let name = tool.name().to_string();

        if self.tools.contains_key(&name) {
            return Err(crate::protocol::GlyphError::JsonRpc(
                format!("Tool '{}' is already registered", name)
            ));
        }

        self.tools.insert(name, tool);
        Ok(())
    }

    pub async fn unregister(&mut self, name: &str) -> Result<()> {
        if self.tools.remove(name).is_none() {
            return Err(crate::protocol::GlyphError::JsonRpc(
                format!("Tool '{}' not found", name)
            ));
        }
        Ok(())
    }

    pub async fn list_tools(&self) -> Result<Vec<ToolDefinition>> {
        let mut tools = Vec::new();

        for tool in self.tools.values() {
            tools.push(ToolDefinition {
                name: tool.name().to_string(),
                description: tool.description().map(|s| s.to_string()),
                input_schema: tool.input_schema(),
            });
        }

        // Sort by name for consistent ordering
        tools.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(tools)
    }

    pub async fn call_tool(&self, request: CallToolRequest) -> Result<CallToolResult> {
        let tool = self.tools.get(&request.name)
            .ok_or_else(|| McpError::tool_not_found(&request.name))?;

        tool.call(request.arguments).await
            .map_err(|e| McpError::tool_execution_error(format!("Tool execution failed: {}", e)))
    }

    pub fn get_tool(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Built-in tools
pub struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> Option<&str> {
        Some("Echo back the input message")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "message".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The message to echo back"
            })
        );

        ToolInputSchema::object()
            .with_properties(properties)
            .with_required(vec!["message".to_string()])
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult> {
        let args = args.unwrap_or(Value::Null);

        let message = args
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("No message provided");

        Ok(CallToolResult::success(vec![Content::text(message)]))
    }
}

pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> Option<&str> {
        Some("Read the contents of a file")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "path".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The path to the file to read"
            })
        );

        ToolInputSchema::object()
            .with_properties(properties)
            .with_required(vec!["path".to_string()])
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult> {
        let args = args.unwrap_or(Value::Null);

        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("Missing 'path' parameter"))?;

        match tokio::fs::read_to_string(path).await {
            Ok(contents) => Ok(CallToolResult::success(vec![Content::text(contents)])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to read file '{}': {}", path, e
            ))])),
        }
    }
}

pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> Option<&str> {
        Some("Write content to a file")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "path".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The path to the file to write"
            })
        );
        properties.insert(
            "content".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The content to write to the file"
            })
        );

        ToolInputSchema::object()
            .with_properties(properties)
            .with_required(vec!["path".to_string(), "content".to_string()])
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult> {
        let args = args.unwrap_or(Value::Null);

        let path = args
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("Missing 'path' parameter"))?;

        let content = args
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("Missing 'content' parameter"))?;

        match tokio::fs::write(path, content).await {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Successfully wrote {} bytes to '{}'", content.len(), path
            ))])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to write to file '{}': {}", path, e
            ))])),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_registry() {
        let mut registry = ToolRegistry::new();

        // Test registration
        registry.register(Box::new(EchoTool)).await.unwrap();
        assert_eq!(registry.len(), 1);
        assert!(registry.get_tool("echo").is_some());

        // Test listing tools
        let tools = registry.list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "echo");

        // Test tool call
        let request = CallToolRequest {
            name: "echo".to_string(),
            arguments: Some(serde_json::json!({"message": "Hello, World!"})),
        };

        let result = registry.call_tool(request).await.unwrap();
        assert_eq!(result.content.len(), 1);

        if let Content::Text { text } = &result.content[0] {
            assert_eq!(text, "Hello, World!");
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_echo_tool() {
        let tool = EchoTool;

        let result = tool.call(Some(serde_json::json!({
            "message": "Test message"
        }))).await.unwrap();

        assert_eq!(result.content.len(), 1);
        if let Content::Text { text } = &result.content[0] {
            assert_eq!(text, "Test message");
        } else {
            panic!("Expected text content");
        }
    }
}