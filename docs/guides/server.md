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
use glyph::server::ServerBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create server with WebSocket transport
    let server = ServerBuilder::new()
        .with_server_info("my-server", "1.0.0")
        .build()
        .await?;

    // Server includes built-in tools by default
    server.run().await?;

    Ok(())
}
```

### Server Capabilities

Configure what your server supports:

```rust
let server = ServerBuilder::new()
    .with_server_info("my-server", "1.0.0")
    .with_tools()           // Enable tool execution (default)
    .with_resources()       // Enable resource access
    .with_prompts()         // Enable prompt templates
    .build()
    .await?;
```

### Transport Options

#### WebSocket Transport (Default)

```rust
use glyph::server::ServerBuilder;
use glyph::transport::WebSocketTransport;

let server = ServerBuilder::new()
    .with_server_info("my-server", "1.0.0")
    .for_websocket("127.0.0.1:7331")
    .await?;
```

#### Stdio Transport

```rust
use glyph::server::ServerBuilder;

let server = ServerBuilder::new()
    .with_server_info("my-server", "1.0.0")
    .for_stdio();
```

## Building Tools

### Basic Tool Implementation

```rust
use glyph::server::Tool;
use glyph::protocol::{CallToolResult, Content, ToolInputSchema};
use async_trait::async_trait;
use serde_json::json;

struct HelloTool;

#[async_trait]
impl Tool for HelloTool {
    fn name(&self) -> &str {
        "hello"
    }

    fn description(&self) -> Option<&str> {
        Some("Say hello to someone")
    }

    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object()
            .with_property("name", json!({
                "type": "string",
                "description": "Name to greet"
            }))
            .with_required(vec!["name"])
    }

    async fn call(&self, args: Option<serde_json::Value>) -> glyph::Result<CallToolResult> {
        let args = args.unwrap_or(json!({}));
        let name = args["name"].as_str().unwrap_or("World");

        Ok(CallToolResult::success(vec![
            Content::text(format!("Hello, {}!", name))
        ]))
    }
}
```

### Registering Tools

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut server = ServerBuilder::new()
        .with_server_info("my-server", "1.0.0")
        .build()
        .await?;

    // Register custom tool
    server.register_tool(HelloTool).await?;

    server.run().await?;
    Ok(())
}
```

### Advanced Tool Features

#### Error Handling

```rust
async fn call(&self, args: Option<serde_json::Value>) -> glyph::Result<CallToolResult> {
    let args = args.unwrap_or(json!({}));
    let path = args["path"].as_str()
        .ok_or_else(|| glyph::Error::invalid_params("Missing 'path' parameter"))?;

    match tokio::fs::read_to_string(path).await {
        Ok(content) => Ok(CallToolResult::success(vec![Content::text(content)])),
        Err(e) => Ok(CallToolResult::error(vec![Content::text(format!("Error: {}", e))]))
    }
}
```

#### Async Operations

```rust
async fn call(&self, args: Option<serde_json::Value>) -> glyph::Result<CallToolResult> {
    let args = args.unwrap_or(json!({}));
    let url = args["url"].as_str().unwrap();

    let response = reqwest::get(url).await?;
    let text = response.text().await?;

    Ok(CallToolResult::success(vec![Content::text(text)]))
}
```

## Managing Resources

### Basic Resource Provider

```rust
use glyph::server::ResourceProvider;
use glyph::protocol::{Resource, ResourceContents};
use async_trait::async_trait;

struct FileSystemProvider;

#[async_trait]
impl ResourceProvider for FileSystemProvider {
    async fn list_resources(&self) -> glyph::Result<Vec<Resource>> {
        // Return list of available resources
        Ok(vec![
            Resource {
                uri: "file:///etc/hosts".to_string(),
                name: "hosts".to_string(),
                description: Some("System hosts file".to_string()),
                mime_type: Some("text/plain".to_string()),
            }
        ])
    }

    async fn read_resource(&self, uri: &str) -> glyph::Result<Vec<ResourceContents>> {
        if uri == "file:///etc/hosts" {
            let content = tokio::fs::read_to_string("/etc/hosts").await?;
            Ok(vec![ResourceContents::text(content)])
        } else {
            Err(glyph::Error::invalid_params("Resource not found"))
        }
    }
}
```

### Registering Resources

```rust
let mut server = ServerBuilder::new()
    .with_server_info("my-server", "1.0.0")
    .with_resources()
    .build()
    .await?;

server.register_resource_provider(FileSystemProvider).await?;
```

## Creating Prompts

### Basic Prompt Template

```rust
use glyph::server::PromptProvider;
use glyph::protocol::{Prompt, PromptMessage, PromptArgument};
use async_trait::async_trait;

struct CodeReviewPrompt;

#[async_trait]
impl PromptProvider for CodeReviewPrompt {
    async fn list_prompts(&self) -> glyph::Result<Vec<Prompt>> {
        Ok(vec![
            Prompt {
                name: "code_review".to_string(),
                description: Some("Review code changes".to_string()),
                arguments: Some(vec![
                    PromptArgument {
                        name: "language".to_string(),
                        description: Some("Programming language".to_string()),
                        required: Some(false),
                    }
                ]),
            }
        ])
    }

    async fn get_prompt(&self, name: &str, args: Option<std::collections::HashMap<String, String>>)
        -> glyph::Result<Vec<PromptMessage>> {
        if name == "code_review" {
            let language = args.as_ref()
                .and_then(|a| a.get("language"))
                .unwrap_or("general");

            Ok(vec![
                PromptMessage::user(format!("Please review this {} code for best practices:", language)),
                PromptMessage::assistant("I'll help review the code. Please provide the code snippet.")
            ])
        } else {
            Err(glyph::Error::invalid_params("Prompt not found"))
        }
    }
}
```

### Registering Prompts

```rust
let mut server = ServerBuilder::new()
    .with_server_info("my-server", "1.0.0")
    .with_prompts()
    .build()
    .await?;

server.register_prompt_provider(CodeReviewPrompt).await?;
```

## Transport Configuration

### WebSocket Configuration

```rust
use glyph::transport::{WebSocketTransport, TransportConfig};

let config = TransportConfig::default()
    .with_max_connections(1000)
    .with_timeout(std::time::Duration::from_secs(30));

let server = ServerBuilder::new()
    .with_server_info("my-server", "1.0.0")
    .for_websocket_with_config("127.0.0.1:7331", config)
    .await?;
```

### Stdio Configuration

```rust
let server = ServerBuilder::new()
    .with_server_info("my-server", "1.0.0")
    .for_stdio();
```

## Error Handling

### Custom Error Types

```rust
use glyph::{Error, Result};
use glyph::protocol::McpError;

async fn call(&self, args: Option<serde_json::Value>) -> Result<CallToolResult> {
    // Validation error
    let path = args.as_ref()
        .and_then(|a| a["path"].as_str())
        .ok_or_else(|| Error::invalid_params("Missing path parameter"))?;

    // Business logic error
    if !std::path::Path::new(path).exists() {
        return Ok(CallToolResult::error(vec![
            Content::text("File does not exist")
        ]));
    }

    // System error
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| Error::internal_error(format!("Failed to read file: {}", e)))?;

    Ok(CallToolResult::success(vec![Content::text(content)]))
}
```

## Advanced Patterns

### Middleware

```rust
use glyph::server::middleware::{Middleware, MiddlewareContext};

struct LoggingMiddleware;

#[async_trait]
impl Middleware for LoggingMiddleware {
    async fn before_request(&self, ctx: &mut MiddlewareContext) -> Result<()> {
        tracing::info!("Request: {} {}", ctx.request.method, ctx.request.id);
        Ok(())
    }

    async fn after_request(&self, ctx: &mut MiddlewareContext) -> Result<()> {
        tracing::info!("Response: {} {}", ctx.request.method, ctx.request.id);
        Ok(())
    }
}

let server = ServerBuilder::new()
    .with_server_info("my-server", "1.0.0")
    .with_middleware(LoggingMiddleware)
    .build()
    .await?;
```

### Session Management

```rust
use glyph::server::SessionManager;
use std::collections::HashMap;

// Custom session data
struct MySessionData {
    user_id: String,
    permissions: Vec<String>,
}

let session_manager = SessionManager::new();

// Sessions are automatically managed by the server
```

## Testing Your Server

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hello_tool() {
        let tool = HelloTool;
        let args = json!({"name": "Alice"});

        let result = tool.call(Some(args)).await.unwrap();
        assert!(result.is_success());

        let content = &result.content[0];
        match content {
            Content::Text { text } => assert_eq!(text, "Hello, Alice!"),
            _ => panic!("Expected text content"),
        }
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_server_integration() {
    let server = ServerBuilder::new()
        .with_server_info("test-server", "1.0.0")
        .for_stdio();

    // Test MCP protocol handshake
    // ... integration test code ...
}
```

## Deployment

### Binary Deployment

```rust
// Build optimized binary
cargo build --release

// Run the server
./target/release/glyph serve
```

### Library Integration

```rust
// Use as a library in your application
let server = ServerBuilder::new()
    .with_server_info("my-app", env!("CARGO_PKG_VERSION"))
    .build()
    .await?;

tokio::spawn(async move {
    server.run().await.unwrap();
});
```

## Next Steps

- [Built-in Tools](tools.md) - Available tools in the binary
- [Transport Guide](transports.md) - Advanced transport options
- [Client Guide](client.md) - Creating MCP clients