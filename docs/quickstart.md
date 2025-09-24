# Quick Start Guide

Get up and running with Glyph in just a few minutes! This guide covers both using the pre-built binary and building custom MCP servers.

## Option 1: Using the Glyph Binary (Fastest)

The Glyph binary comes with 7 built-in tools and is ready to use immediately.

### Installation

```bash
# Install from crates.io (when published)
cargo install glyph

# Or build from source
git clone https://github.com/ghostkellz/glyph
cd glyph
cargo build --release
```

### Start the Server

```bash
# Start WebSocket server (default)
./target/release/glyph serve

# Start with verbose logging
./target/release/glyph serve --verbose

# Custom address and port
./target/release/glyph serve --address 0.0.0.0:8080

# Use stdio transport
./target/release/glyph serve --transport stdio
```

### Built-in Tools

The binary includes 7 production-ready tools:

- **echo**: Echo back input messages
- **read_file**: Read file contents
- **write_file**: Write content to files
- **list_directory**: List directory contents
- **delete_file**: Delete files or directories
- **shell_execute**: Execute shell commands
- **http_request**: Make HTTP requests to external APIs

### Test the Server

```bash
# Build and run test client
cargo run --example test_client
```

## Option 2: Building Custom Servers

For custom MCP servers, use the Glyph library:

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
glyph = { git = "https://github.com/ghostkellz/glyph", tag = "v0.1.0-rc.1" }
tokio = { version = "1", features = ["full"] }
serde_json = "1.0"
```

### Simple Server with Built-in Tools

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

    // Server starts with built-in tools by default
    server.run().await?;

    Ok(())
}
```

### Custom Tool Server

```rust
use glyph::server::{ServerBuilder, Tool, CallToolResult, Content};
use glyph::protocol::ToolInputSchema;
use async_trait::async_trait;
use serde_json::json;

// Define a custom tool
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let mut server = ServerBuilder::new()
        .with_server_info("hello-server", "1.0.0")
        .build()
        .await?;

    // Register custom tool
    server.register_tool(HelloTool).await?;

    server.run().await?;
    Ok(())
}
```

## Testing Your Server

### WebSocket Transport

```bash
# Start your server
cargo run

# In another terminal, test with curl (basic connectivity)
curl -X GET http://127.0.0.1:7331/health  # If you add health endpoint

# Use MCP client or test client
cargo run --example test_client
```

### Stdio Transport

```bash
# Test stdio server
echo '{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "clientInfo": {"name": "test", "version": "1.0"}}}' | cargo run --bin your_server
```

## Next Steps

- **Using the binary**: See [Binary Usage Guide](guides/binary.md)
- **Building servers**: Read the [Server Guide](guides/server.md)
- **Available tools**: Check [Built-in Tools](guides/tools.md)
- **Client integration**: See [Client Examples](examples/basic.md)

## Troubleshooting

### Server won't start
- Check if port 7331 is available
- Try a different port: `glyph serve --address 127.0.0.1:8080`

### Client can't connect
- Verify server is running: `netstat -tlnp | grep 7331`
- Check firewall settings
- Try localhost instead of 127.0.0.1

### Build errors
- Ensure Rust 1.75+ is installed
- Run `cargo update` to update dependencies
- Check [troubleshooting guide](troubleshooting.md)