# Quick Start Guide

Get up and running with Glyph in just a few minutes! This guide will walk you through creating your first MCP server and client.

## Installation

Add Glyph to your `Cargo.toml`:

```toml
[dependencies]
glyph = { git = "https://github.com/ghostkellz/glyph", tag = "v0.1.0" }
tokio = { version = "1", features = ["full"] }
```

## Your First MCP Server

Let's create a simple file server that can read and write files:

```rust
use glyph::{
    Server, Tool, ToolInputSchema, CallToolResult, Content,
    async_trait, Result, json,
};
use std::collections::HashMap;

// Define a tool for reading files
struct ReadFileTool;

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
            json!({
                "type": "string",
                "description": "The path to the file to read"
            })
        );

        ToolInputSchema::object()
            .with_properties(properties)
            .with_required(vec!["path".to_string()])
    }

    async fn call(&self, args: Option<serde_json::Value>) -> Result<CallToolResult> {
        let args = args.unwrap_or(json!({}));
        let path = args["path"].as_str()
            .ok_or_else(|| glyph::McpError::invalid_params("Missing 'path' parameter"))?;

        match tokio::fs::read_to_string(path).await {
            Ok(contents) => Ok(CallToolResult::success(vec![Content::text(contents)])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Failed to read file: {}", e
            ))])),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::init();

    // Create server with stdio transport
    let server = Server::builder()
        .with_server_info("file-server", "1.0.0")
        .with_tools()
        .build();

    // Register our tool
    server.register_tool(ReadFileTool).await?;

    // Run the server
    server.builder().for_stdio().run().await
}
```

## Your First MCP Client

Now let's create a client that can call our file server:

```rust
use glyph::{Client, json};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to the server via stdio
    let client = Client::builder()
        .with_client_info("file-client", "1.0.0")
        .connect_stdio()
        .await?;

    // List available tools
    let tools = client.tools().list_tools(None).await?;
    println!("Available tools: {:?}", tools.tools);

    // Call the read_file tool
    let result = client.tools().call_tool(
        "read_file",
        Some(json!({ "path": "README.md" }))
    ).await?;

    // Print the result
    for content in result.content {
        if let glyph::Content::Text { text } = content {
            println!("File contents:\n{}", text);
        }
    }

    Ok(())
}
```

## Running Your First Example

1. **Save the server code** to `examples/file_server.rs`
2. **Save the client code** to `examples/file_client.rs`
3. **Add to Cargo.toml**:
   ```toml
   [[example]]
   name = "file_server"
   path = "examples/file_server.rs"

   [[example]]
   name = "file_client"
   path = "examples/file_client.rs"
   ```

4. **Run the server**:
   ```bash
   cargo run --example file_server
   ```

5. **In another terminal, run the client**:
   ```bash
   echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"read_file","arguments":{"path":"README.md"}}}' | cargo run --example file_server
   ```

## WebSocket Example

For real-time applications, use WebSocket transport:

### Server
```rust
#[tokio::main]
async fn main() -> Result<()> {
    let server = Server::builder()
        .with_tools()
        .build();

    server.register_tool(ReadFileTool).await?;

    // Run WebSocket server on localhost:8080
    server.builder()
        .for_websocket("127.0.0.1:8080")
        .await?
        .run()
        .await
}
```

### Client
```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::connect_websocket("ws://127.0.0.1:8080").await?;

    let result = client.tools().call_tool(
        "read_file",
        Some(json!({ "path": "Cargo.toml" }))
    ).await?;

    println!("Result: {:?}", result);
    Ok(())
}
```

## Built-in Tools

Glyph comes with several built-in tools:

```rust
use glyph::server::{EchoTool, ReadFileTool, WriteFileTool};

let server = Server::builder()
    .with_tools()
    .build();

// Register built-in tools
server.register_tool(EchoTool).await?;
server.register_tool(ReadFileTool).await?;
server.register_tool(WriteFileTool).await?;
```

## Resource Example

Expose files as resources:

```rust
use glyph::server::{FileSystemResourceProvider, ResourceProvider};

let server = Server::builder()
    .with_resources()
    .build();

// Expose current directory as resources
let provider = FileSystemResourceProvider::new(".")
    .with_allowed_extensions(vec!["txt".to_string(), "md".to_string()]);

server.register_resource_provider(provider).await?;
```

## Client Resource Access

```rust
let client = Client::connect_stdio().await?;

// List available resources
let resources = client.resources().list_resources(None).await?;
println!("Available resources: {:?}", resources.resources);

// Read a specific resource
let content = client.resources().read_resource_text("file://README.md").await?;
println!("Resource content: {}", content);
```

## Error Handling

Glyph provides comprehensive error types:

```rust
use glyph::{GlyphError, McpError};

match client.tools().call_tool("nonexistent", None).await {
    Ok(result) => println!("Success: {:?}", result),
    Err(GlyphError::Mcp(McpError { code, message, .. })) => {
        println!("MCP Error {}: {}", code, message);
    }
    Err(e) => println!("Other error: {}", e),
}
```

## Next Steps

- **[Server Guide](guides/server.md)** - Learn about advanced server features
- **[Client Guide](guides/client.md)** - Explore client capabilities
- **[Transport Guide](guides/transports.md)** - Choose the right transport
- **[Tools Guide](guides/tools.md)** - Create sophisticated tools
- **[Examples](examples/basic.md)** - See more detailed examples

## Common Patterns

### Request/Response Pattern
```rust
// Server responds to individual requests
let result = client.tools().call_tool("my_tool", Some(args)).await?;
```

### Streaming Pattern
```rust
// For long-running operations with progress updates
// (Implementation depends on your specific use case)
```

### Batch Operations
```rust
// Process multiple requests efficiently
let tools = vec!["tool1", "tool2", "tool3"];
let futures: Vec<_> = tools.iter()
    .map(|name| client.tools().call_tool(*name, None))
    .collect();

let results = futures::future::join_all(futures).await;
```

That's it! You now have a working MCP server and client. Explore the other guides to learn about advanced features like authentication, observability, and integration with your existing systems.