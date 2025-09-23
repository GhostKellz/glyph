# Server Guide

This guide covers everything you need to know about building MCP servers with Glyph, from basic concepts to advanced patterns.

## Table of Contents

- [Server Basics](#server-basics)
- [Building Tools](#building-tools)
- [Managing Resources](#managing-resources)
- [Creating Prompts](#creating-prompts)
- [Transport Configuration](#transport-configuration)
- [Session Management](#session-management)
- [Error Handling](#error-handling)
- [Advanced Patterns](#advanced-patterns)

## Server Basics

### Creating a Server

The simplest way to create a server is using the builder pattern:

```rust
use glyph::Server;

#[tokio::main]
async fn main() -> glyph::Result<()> {
    let server = Server::builder()
        .with_server_info("my-server", "1.0.0")
        .with_tools()
        .with_resources()
        .with_prompts()
        .build();

    // Run with stdio transport
    server.builder().for_stdio().run().await
}
```

### Server Capabilities

Declare what your server supports:

```rust
let server = Server::builder()
    .with_tools()                    // Enable tool execution
    .with_tool_list_changes()        // Notify clients when tools change
    .with_resources()                // Enable resource access
    .with_resource_subscriptions()   // Support resource subscriptions
    .with_prompts()                  // Enable prompt templates
    .with_prompt_list_changes()      // Notify when prompts change
    .build();
```

### Transport Options

Choose the transport that fits your use case:

```rust
// stdio - for CLI tools and subprocesses
server.builder().for_stdio().run().await?;

// WebSocket - for real-time applications
server.builder()
    .for_websocket("127.0.0.1:8080")
    .await?
    .run()
    .await?;

// Custom transport
let transport = MyCustomTransport::new();
server.run_with_transport(transport).await?;
```

## Building Tools

Tools are the core functionality of your MCP server. They execute actions based on client requests.

### Simple Tool Example

```rust
use glyph::{Tool, ToolInputSchema, CallToolResult, Content, async_trait};
use std::collections::HashMap;

struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculate"
    }

    fn description(&self) -> Option<&str> {
        Some("Perform basic arithmetic calculations")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut properties = HashMap::new();

        properties.insert("expression".to_string(), glyph::json!({
            "type": "string",
            "description": "Mathematical expression to evaluate",
            "examples": ["2 + 2", "10 * 5", "sqrt(16)"]
        }));

        ToolInputSchema::object()
            .with_properties(properties)
            .with_required(vec!["expression".to_string()])
    }

    async fn call(&self, args: Option<serde_json::Value>) -> glyph::Result<CallToolResult> {
        let args = args.unwrap_or_default();

        let expression = args["expression"]
            .as_str()
            .ok_or_else(|| glyph::McpError::invalid_params("Missing expression"))?;

        match evaluate_expression(expression) {
            Ok(result) => Ok(CallToolResult::success(vec![
                Content::text(format!("{} = {}", expression, result))
            ])),
            Err(e) => Ok(CallToolResult::error(vec![
                Content::text(format!("Error: {}", e))
            ])),
        }
    }
}

fn evaluate_expression(expr: &str) -> Result<f64, String> {
    // Simple calculator implementation
    match expr {
        "2 + 2" => Ok(4.0),
        "10 * 5" => Ok(50.0),
        _ => Err("Unsupported expression".to_string()),
    }
}

// Register the tool
server.register_tool(CalculatorTool).await?;
```

### Tool with Typed Parameters

For better type safety, create structured parameter types:

```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct FileReadParams {
    path: String,
    encoding: Option<String>,
}

#[derive(Serialize)]
struct FileReadResult {
    content: String,
    size: usize,
    encoding: String,
}

struct TypedFileReader;

#[async_trait]
impl Tool for TypedFileReader {
    fn name(&self) -> &str {
        "read_file_typed"
    }

    fn input_schema(&self) -> ToolInputSchema {
        // You can generate this from the struct using serde_json
        let schema = glyph::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                },
                "encoding": {
                    "type": "string",
                    "description": "Text encoding (default: utf-8)",
                    "default": "utf-8"
                }
            },
            "required": ["path"]
        });

        serde_json::from_value(schema).unwrap()
    }

    async fn call(&self, args: Option<serde_json::Value>) -> glyph::Result<CallToolResult> {
        let params: FileReadParams = match args {
            Some(value) => serde_json::from_value(value)?,
            None => return Err(glyph::McpError::invalid_params("Missing parameters").into()),
        };

        let encoding = params.encoding.unwrap_or_else(|| "utf-8".to_string());

        match tokio::fs::read_to_string(&params.path).await {
            Ok(content) => {
                let result = FileReadResult {
                    size: content.len(),
                    content,
                    encoding,
                };

                // Return structured data in metadata
                let mut call_result = CallToolResult::success(vec![
                    Content::text(format!("Read {} bytes from {}", result.size, params.path))
                ]);
                call_result = call_result.with_meta(glyph::json!({
                    "structured_result": result
                }));

                Ok(call_result)
            }
            Err(e) => Ok(CallToolResult::error(vec![
                Content::text(format!("Failed to read {}: {}", params.path, e))
            ])),
        }
    }
}
```

### Tool with Progress Updates

For long-running operations, you can send progress updates:

```rust
struct LongRunningTool;

#[async_trait]
impl Tool for LongRunningTool {
    fn name(&self) -> &str {
        "process_data"
    }

    async fn call(&self, args: Option<serde_json::Value>) -> glyph::Result<CallToolResult> {
        let total_steps = 100;

        for i in 0..total_steps {
            // Simulate work
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

            // In a real implementation, you'd send progress notifications
            // through the server's notification system
            if i % 10 == 0 {
                tracing::info!("Progress: {}/{}", i, total_steps);
            }
        }

        Ok(CallToolResult::success(vec![
            Content::text("Processing completed successfully")
        ]))
    }
}
```

### Built-in Tools

Glyph provides several built-in tools you can use:

```rust
use glyph::server::{EchoTool, ReadFileTool, WriteFileTool};

// Register built-in tools
server.register_tool(EchoTool).await?;
server.register_tool(ReadFileTool).await?;
server.register_tool(WriteFileTool).await?;
```

## Managing Resources

Resources provide read-only access to data sources like files, databases, or APIs.

### File System Resources

Expose a directory tree as resources:

```rust
use glyph::server::FileSystemResourceProvider;

let provider = FileSystemResourceProvider::new("/path/to/data")
    .with_allowed_extensions(vec![
        "txt".to_string(),
        "md".to_string(),
        "json".to_string(),
    ]);

server.register_resource_provider(provider).await?;
```

### Custom Resource Provider

Create your own resource provider:

```rust
use glyph::{ResourceProvider, Resource, ResourceContents, async_trait};

struct DatabaseResourceProvider {
    connection_string: String,
}

#[async_trait]
impl ResourceProvider for DatabaseResourceProvider {
    async fn list_resources(&self) -> glyph::Result<Vec<Resource>> {
        // Query database for available tables/views
        let resources = vec![
            Resource::new("db://users", "users")
                .with_description("User database table")
                .with_mime_type("application/json"),
            Resource::new("db://orders", "orders")
                .with_description("Order database table")
                .with_mime_type("application/json"),
        ];

        Ok(resources)
    }

    async fn read_resource(&self, uri: &str) -> glyph::Result<Vec<ResourceContents>> {
        match uri {
            "db://users" => {
                let data = query_users_table(&self.connection_string).await?;
                Ok(vec![ResourceContents::text_with_mime_type(
                    uri,
                    serde_json::to_string_pretty(&data)?,
                    "application/json".to_string(),
                )])
            }
            "db://orders" => {
                let data = query_orders_table(&self.connection_string).await?;
                Ok(vec![ResourceContents::text_with_mime_type(
                    uri,
                    serde_json::to_string_pretty(&data)?,
                    "application/json".to_string(),
                )])
            }
            _ => Err(glyph::McpError::resource_not_found(uri).into()),
        }
    }
}

async fn query_users_table(conn: &str) -> glyph::Result<serde_json::Value> {
    // Database query implementation
    Ok(glyph::json!([
        {"id": 1, "name": "Alice", "email": "alice@example.com"},
        {"id": 2, "name": "Bob", "email": "bob@example.com"}
    ]))
}

async fn query_orders_table(conn: &str) -> glyph::Result<serde_json::Value> {
    Ok(glyph::json!([
        {"id": 1, "user_id": 1, "total": 29.99},
        {"id": 2, "user_id": 2, "total": 45.50}
    ]))
}

// Register the provider
let provider = DatabaseResourceProvider {
    connection_string: "postgresql://localhost/mydb".to_string(),
};
server.register_resource_provider(provider).await?;
```

### Memory Resources

For dynamic or computed resources:

```rust
use glyph::server::MemoryResourceProvider;

let mut provider = MemoryResourceProvider::new();

// Add some resources
provider.add_resource(
    "memory://config".to_string(),
    serde_json::to_string_pretty(&my_config)?,
    Some("application/json".to_string()),
);

provider.add_resource(
    "memory://status".to_string(),
    get_system_status(),
    Some("text/plain".to_string()),
);

server.register_resource_provider(provider).await?;
```

## Creating Prompts

Prompts are reusable templates that can be customized with arguments.

### Simple Prompt

```rust
use glyph::server::SimplePrompt;

let prompt = SimplePrompt::new(
    "code_review",
    "Please review this code for best practices and potential issues:\n\n```{language}\n{code}\n```"
)
.with_description("Generate a code review for the provided code")
.with_argument("code", Some("The code to review".to_string()), true)
.with_argument("language", Some("Programming language".to_string()), false);

server.register_prompt(prompt).await?;
```

### Complex Prompt Provider

```rust
use glyph::{PromptProvider, GetPromptResult, PromptMessage, PromptRole, Content, async_trait};

struct DocumentationPrompt;

#[async_trait]
impl PromptProvider for DocumentationPrompt {
    fn name(&self) -> &str {
        "generate_docs"
    }

    fn description(&self) -> Option<&str> {
        Some("Generate documentation for code")
    }

    fn arguments(&self) -> Vec<glyph::PromptArgument> {
        vec![
            glyph::PromptArgument {
                name: "code".to_string(),
                description: Some("Code to document".to_string()),
                required: Some(true),
            },
            glyph::PromptArgument {
                name: "style".to_string(),
                description: Some("Documentation style (api, tutorial, reference)".to_string()),
                required: Some(false),
            },
        ]
    }

    async fn get_prompt(&self, args: HashMap<String, String>) -> glyph::Result<GetPromptResult> {
        let code = args.get("code")
            .ok_or_else(|| glyph::McpError::invalid_params("Missing code argument"))?;

        let style = args.get("style").map(|s| s.as_str()).unwrap_or("api");

        let template = match style {
            "tutorial" => include_str!("../templates/tutorial_docs.md"),
            "reference" => include_str!("../templates/reference_docs.md"),
            _ => include_str!("../templates/api_docs.md"),
        };

        let prompt = template.replace("{code}", code);

        Ok(GetPromptResult {
            description: Some(format!("Generate {} documentation", style)),
            messages: vec![PromptMessage {
                role: PromptRole::User,
                content: Content::text(prompt),
            }],
        })
    }
}

server.register_prompt(DocumentationPrompt).await?;
```

## Transport Configuration

### stdio Configuration

```rust
use glyph::TransportConfig;

let config = TransportConfig::new()
    .with_read_timeout(std::time::Duration::from_secs(30))
    .with_write_timeout(std::time::Duration::from_secs(10))
    .with_max_message_size(1024 * 1024); // 1MB

let server = Server::builder()
    .with_transport_config(config)
    .build();
```

### WebSocket Configuration

```rust
let server = Server::builder()
    .with_transport_config(
        TransportConfig::new()
            .with_ping_interval(std::time::Duration::from_secs(30))
            .with_ping_timeout(std::time::Duration::from_secs(10))
    )
    .build();

let ws_server = server.builder()
    .for_websocket("0.0.0.0:8080")
    .await?;

println!("Server listening on {}", ws_server.local_addr()?);
ws_server.run().await?;
```

## Session Management

Track client sessions and their capabilities:

```rust
// Sessions are automatically managed, but you can access session info
// within tools and other handlers

struct SessionAwareTool;

#[async_trait]
impl Tool for SessionAwareTool {
    fn name(&self) -> &str {
        "get_session_info"
    }

    async fn call(&self, args: Option<serde_json::Value>) -> glyph::Result<CallToolResult> {
        // In a real implementation, you'd have access to session context
        // through tool context or request metadata

        let session_info = glyph::json!({
            "client_name": "example-client",
            "capabilities": ["tools", "resources"],
            "connected_at": "2024-01-01T00:00:00Z"
        });

        Ok(CallToolResult::success(vec![
            Content::text(format!("Session info: {}", session_info))
        ]))
    }
}
```

## Error Handling

### Proper Error Responses

```rust
impl Tool for SafeTool {
    async fn call(&self, args: Option<serde_json::Value>) -> glyph::Result<CallToolResult> {
        match risky_operation().await {
            Ok(result) => Ok(CallToolResult::success(vec![
                Content::text(format!("Success: {}", result))
            ])),
            Err(e) => {
                // Log the error for debugging
                tracing::error!("Tool execution failed: {}", e);

                // Return user-friendly error
                Ok(CallToolResult::error(vec![
                    Content::text(format!("Operation failed: {}", e))
                ]))
            }
        }
    }
}

async fn risky_operation() -> Result<String, String> {
    // Some operation that might fail
    Err("Something went wrong".to_string())
}
```

### Custom Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
enum MyToolError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("External service error: {0}")]
    ExternalService(String),

    #[error("Rate limit exceeded")]
    RateLimit,
}

impl From<MyToolError> for glyph::GlyphError {
    fn from(err: MyToolError) -> Self {
        glyph::GlyphError::JsonRpc(err.to_string())
    }
}
```

## Advanced Patterns

### Tool Composition

Create tools that call other tools:

```rust
struct CompositeAnalysisTool {
    tools: Arc<ToolRegistry>,
}

#[async_trait]
impl Tool for CompositeAnalysisTool {
    fn name(&self) -> &str {
        "analyze_file"
    }

    async fn call(&self, args: Option<serde_json::Value>) -> glyph::Result<CallToolResult> {
        let args = args.unwrap_or_default();
        let path = args["path"].as_str().unwrap_or("");

        // First, read the file
        let read_request = glyph::CallToolRequest {
            name: "read_file".to_string(),
            arguments: Some(glyph::json!({"path": path})),
        };

        let file_content = self.tools.call_tool(read_request).await?;

        // Then analyze it (pseudocode)
        let analysis = perform_analysis(&file_content).await?;

        Ok(CallToolResult::success(vec![
            Content::text(format!("Analysis complete: {}", analysis))
        ]))
    }
}

async fn perform_analysis(content: &glyph::CallToolResult) -> glyph::Result<String> {
    // Analysis logic
    Ok("File looks good".to_string())
}
```

### Dynamic Tool Registration

```rust
struct DynamicServer {
    server: Server,
}

impl DynamicServer {
    async fn load_plugins(&self, plugin_dir: &str) -> glyph::Result<()> {
        // Scan for plugin files
        let mut entries = tokio::fs::read_dir(plugin_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if entry.path().extension() == Some(std::ffi::OsStr::new("toml")) {
                self.load_plugin_config(&entry.path()).await?;
            }
        }

        Ok(())
    }

    async fn load_plugin_config(&self, config_path: &std::path::Path) -> glyph::Result<()> {
        // Load plugin configuration and register tools dynamically
        let config = tokio::fs::read_to_string(config_path).await?;
        // Parse config and create tools...

        Ok(())
    }
}
```

### Middleware Integration

```rust
use glyph::server::{LoggingMiddleware, TimingMiddleware, MiddlewareStack};

let middleware = MiddlewareStack::new()
    .add(LoggingMiddleware)
    .add(TimingMiddleware);

// Middleware will be integrated into the server in future versions
```

This guide covers the essential patterns for building robust MCP servers with Glyph. For more advanced topics, see the [Advanced Topics](../advanced/) section.