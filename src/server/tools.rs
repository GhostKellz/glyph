use crate::protocol::{
    Content, Tool as ToolDefinition, ToolInputSchema, CallToolRequest, CallToolResult,
    McpError, RequestId,
};
use crate::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use serde_json::Value;
use jsonschema::JSONSchema;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> Option<&str> {
        None
    }
    fn input_schema(&self) -> ToolInputSchema;

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult>;

    // Optional: validate input against schema
    fn validate_input(&self, args: &Value) -> std::result::Result<(), McpError> {
        let schema = self.input_schema();
        let schema_value = serde_json::to_value(&schema)
            .map_err(|e| McpError::internal_error(format!("Schema serialization error: {}", e)))?;

        let compiled = JSONSchema::compile(&schema_value)
            .map_err(|e| McpError::internal_error(format!("Schema compilation error: {}", e)))?;

        compiled.validate(args)
            .map_err(|errors| {
                let error_messages: Vec<String> = errors.map(|e| e.to_string()).collect();
                McpError::invalid_params(format!("Input validation failed: {}", error_messages.join(", ")))
            })?;

        Ok(())
    }
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
            ).into());
        }

        self.tools.insert(name, tool);
        Ok(())
    }

    pub async fn unregister(&mut self, name: &str) -> Result<()> {
        if self.tools.remove(name).is_none() {
            return Err(crate::protocol::GlyphError::JsonRpc(
                format!("Tool '{}' not found", name)
            ).into());
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

        // Validate input against schema if arguments are provided
        if let Some(ref args) = request.arguments {
            tool.validate_input(args)?;
        }

        Ok(tool.call(request.arguments).await?)
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

pub struct ShellExecuteTool;

#[async_trait]
impl Tool for ShellExecuteTool {
    fn name(&self) -> &str {
        "shell_execute"
    }

    fn description(&self) -> Option<&str> {
        Some("Execute a shell command with optional timeout and working directory")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "command".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The shell command to execute"
            })
        );
        properties.insert(
            "cwd".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "Working directory for command execution (optional)"
            })
        );
        properties.insert(
            "timeout_seconds".to_string(),
            serde_json::json!({
                "type": "number",
                "description": "Command timeout in seconds (optional, default 30)",
                "minimum": 1,
                "maximum": 300
            })
        );
        properties.insert(
            "env".to_string(),
            serde_json::json!({
                "type": "object",
                "description": "Environment variables to set (optional)",
                "additionalProperties": {
                    "type": "string"
                }
            })
        );

        ToolInputSchema::object()
            .with_properties(properties)
            .with_required(vec!["command".to_string()])
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult> {
        let args = args.unwrap_or(Value::Null);

        let command = args
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("Missing 'command' parameter"))?;

        let cwd = args
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let timeout_seconds = args
            .get("timeout_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(30);

        let env_vars = args
            .get("env")
            .and_then(|v| v.as_object())
            .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect::<HashMap<_, _>>())
            .unwrap_or_default();

        // Basic sandboxing: restrict to safe commands
        let allowed_commands = ["ls", "cat", "grep", "find", "head", "tail", "wc", "sort", "uniq", "echo"];
        let cmd_base = command.split_whitespace().next().unwrap_or("");

        if !allowed_commands.contains(&cmd_base) {
            return Ok(CallToolResult::error(vec![Content::text(format!(
                "Command '{}' is not allowed for security reasons", cmd_base
            ))]));
        }

        // Execute command
        let mut cmd = tokio::process::Command::new("sh");
        cmd.arg("-c").arg(command);

        if let Some(cwd) = cwd {
            cmd.current_dir(cwd);
        }

        // Set environment variables
        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        // Set timeout
        let timeout_duration = std::time::Duration::from_secs(timeout_seconds);

        match tokio::time::timeout(timeout_duration, cmd.output()).await {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                let mut content = vec![];
                if !stdout.is_empty() {
                    content.push(Content::text(format!("STDOUT:\n{}", stdout)));
                }
                if !stderr.is_empty() {
                    content.push(Content::text(format!("STDERR:\n{}", stderr)));
                }

                let exit_code = output.status.code().unwrap_or(-1);
                content.push(Content::text(format!("Exit code: {}", exit_code)));

                if output.status.success() {
                    Ok(CallToolResult::success(content))
                } else {
                    Ok(CallToolResult::error(content))
                }
            }
            Ok(Err(e)) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Command execution failed: {}", e
            ))])),
            Err(_) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Command timed out after {} seconds", timeout_seconds
            ))])),
        }
    }
}

pub struct ListDirectoryTool;

#[async_trait]
impl Tool for ListDirectoryTool {
    fn name(&self) -> &str {
        "list_directory"
    }

    fn description(&self) -> Option<&str> {
        Some("List contents of a directory")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "path".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The directory path to list"
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

        match tokio::fs::read_dir(path).await {
            Ok(mut entries) => {
                let mut files = Vec::new();
                let mut dirs = Vec::new();

                while let Ok(Some(entry)) = entries.next_entry().await {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    if let Ok(metadata) = entry.metadata().await {
                        if metadata.is_dir() {
                            dirs.push(file_name + "/");
                        } else {
                            files.push(file_name);
                        }
                    }
                }

                let mut content = vec![];
                if !dirs.is_empty() {
                    content.push(Content::text(format!("Directories:\n{}", dirs.join("\n"))));
                }
                if !files.is_empty() {
                    content.push(Content::text(format!("Files:\n{}", files.join("\n"))));
                }

                Ok(CallToolResult::success(content))
            }
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to list directory '{}': {}", path, e
            ))])),
        }
    }
}

pub struct DeleteFileTool;

#[async_trait]
impl Tool for DeleteFileTool {
    fn name(&self) -> &str {
        "delete_file"
    }

    fn description(&self) -> Option<&str> {
        Some("Delete a file or empty directory")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "path".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The path to the file or directory to delete"
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

        let metadata = tokio::fs::metadata(path).await
            .map_err(|e| McpError::invalid_params(format!("Cannot access path '{}': {}", path, e)))?;

        let result = if metadata.is_dir() {
            tokio::fs::remove_dir(path).await
        } else {
            tokio::fs::remove_file(path).await
        };

        match result {
            Ok(_) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Successfully deleted '{}'", path
            ))])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to delete '{}': {}", path, e
            ))])),
        }
    }
}

pub struct HttpClientTool;

#[async_trait]
impl Tool for HttpClientTool {
    fn name(&self) -> &str {
        "http_request"
    }

    fn description(&self) -> Option<&str> {
        Some("Make HTTP requests to external APIs")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "method".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "HTTP method (GET, POST, PUT, DELETE, etc.)",
                "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"]
            })
        );
        properties.insert(
            "url".to_string(),
            serde_json::json!({
                "type": "string",
                "description": "The URL to request"
            })
        );
        properties.insert(
            "headers".to_string(),
            serde_json::json!({
                "type": "object",
                "description": "HTTP headers (optional)",
                "additionalProperties": {
                    "type": "string"
                }
            })
        );
        properties.insert(
            "body".to_string(),
            serde_json::json!({
                "type": ["string", "object", "null"],
                "description": "Request body (optional)"
            })
        );
        properties.insert(
            "timeout_seconds".to_string(),
            serde_json::json!({
                "type": "number",
                "description": "Request timeout in seconds (optional, default 30)",
                "minimum": 1,
                "maximum": 300
            })
        );

        ToolInputSchema::object()
            .with_properties(properties)
            .with_required(vec!["method".to_string(), "url".to_string()])
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult> {
        #[cfg(not(feature = "http"))]
        {
            return Ok(CallToolResult::error(vec![Content::text(
                "HTTP client not available - compile with 'http' feature"
            )]));
        }

        #[cfg(feature = "http")]
        {
            let args = args.unwrap_or(Value::Null);

            let method = args
                .get("method")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::invalid_params("Missing 'method' parameter"))?;

            let url = args
                .get("url")
                .and_then(|v| v.as_str())
                .ok_or_else(|| McpError::invalid_params("Missing 'url' parameter"))?;

            let headers = args
                .get("headers")
                .and_then(|v| v.as_object())
                .map(|obj| obj.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect::<HashMap<_, _>>())
                .unwrap_or_default();

            let body = args.get("body");
            let timeout_seconds = args
                .get("timeout_seconds")
                .and_then(|v| v.as_u64())
                .unwrap_or(30);

            // Build request
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(timeout_seconds))
                .build()
                .map_err(|e| McpError::internal_error(format!("Failed to create HTTP client: {}", e)))?;

            let mut request = match method {
                "GET" => client.get(url),
                "POST" => client.post(url),
                "PUT" => client.put(url),
                "DELETE" => client.delete(url),
                "PATCH" => client.patch(url),
                "HEAD" => client.head(url),
                "OPTIONS" => client.request(reqwest::Method::OPTIONS, url),
                _ => return Ok(CallToolResult::error(vec![Content::text(format!(
                    "Unsupported HTTP method: {}", method
                ))])),
            };

            // Add headers
            for (key, value) in headers {
                request = request.header(&key, &value);
            }

            // Add body if provided
            if let Some(body_value) = body {
                if let Some(body_str) = body_value.as_str() {
                    request = request.body(body_str.to_string());
                } else if body_value.is_object() {
                    request = request
                        .header("Content-Type", "application/json")
                        .body(serde_json::to_string(body_value).unwrap_or_default());
                }
            }

            // Execute request
            match request.send().await {
                Ok(response) => {
                    let status = response.status();
                    let headers = response.headers().clone();
                    let body = response.text().await.unwrap_or_default();

                    let content = vec![
                        Content::text(format!("Status: {}", status)),
                        Content::text(format!("Headers: {:?}", headers)),
                        Content::text(format!("Body: {}", body)),
                    ];

                    if status.is_success() {
                        Ok(CallToolResult::success(content))
                    } else {
                        Ok(CallToolResult::error(content))
                    }
                }
                Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                    "HTTP request failed: {}", e
                ))])),
            }
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