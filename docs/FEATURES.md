# Glyph Features Guide

Comprehensive guide to all Glyph features and how to use them.

---

## Core Features

### 🔧 Tool System

Implement custom tools with type-safe schemas:

```rust
use glyph::server::{Tool, ToolContext};

struct MyTool;

#[async_trait::async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str { "my_tool" }

    async fn call(&self, ctx: &ToolContext, args: Option<Value>) -> Result<CallToolResult> {
        // Your logic here
    }

    fn input_schema(&self) -> ToolInputSchema {
        // JSON Schema for tool inputs
    }
}
```

**Features:**
- JSON Schema validation
- Async execution
- Metadata support
- Error handling

See: [Integration Contract - Tool API](INTEGRATION_CONTRACT.md#tool-api)

---

### 📁 Resource System

Provide read-only access to data:

```rust
use glyph::server::ResourceProvider;

struct MyResourceProvider;

#[async_trait::async_trait]
impl ResourceProvider for MyResourceProvider {
    async fn list(&self) -> Result<Vec<Resource>> {
        // Return available resources
    }

    async fn read(&self, uri: &str) -> Result<ResourceContents> {
        // Read resource by URI
    }
}
```

**Supported content types:**
- Text resources
- Binary resources (blob)
- Custom MIME types

See: [Integration Contract - Resource API](INTEGRATION_CONTRACT.md#resource-api)

---

### 💬 Prompt System

Create reusable prompt templates:

```rust
use glyph::server::PromptProvider;

struct MyPromptProvider;

#[async_trait::async_trait]
impl PromptProvider for MyPromptProvider {
    fn name(&self) -> &str { "my_prompt" }

    async fn get(&self, args: Option<Value>) -> Result<Prompt> {
        // Generate prompt with arguments
    }
}
```

**Features:**
- Variable substitution
- Multi-turn conversations
- Role-based messages (System, User, Assistant)

See: [Integration Contract - Prompt API](INTEGRATION_CONTRACT.md#prompt-api)

---

## Transport Options

### Stdio Transport

**Best for:** CLI tools, local development

```rust
let server = Server::builder()
    .for_stdio()
    .await?;
```

**Usage:**
```bash
glyph serve --transport stdio
```

---

### WebSocket Transport

**Best for:** Web apps, remote access

```rust
let server = Server::builder()
    .for_websocket("127.0.0.1:7331")
    .await?;
```

**Usage:**
```bash
glyph serve --address 127.0.0.1:7331
```

**Client connection:**
```javascript
const ws = new WebSocket('ws://localhost:7331');
```

---

### HTTP Transport

**Best for:** REST APIs, webhooks (planned for v0.2)

```rust
let server = Server::builder()
    .for_http("0.0.0.0:8080")
    .await?;
```

---

## Configuration

### Server Builder Options

```rust
use glyph::server::Server;

let server = Server::builder()
    .with_server_info("my-server", "1.0.0")
    .with_capabilities(ServerCapabilities {
        tools: Some(ToolsCapability { list_changed: true }),
        resources: Some(ResourcesCapability { subscribe: false, list_changed: true }),
        prompts: Some(PromptsCapability { list_changed: true }),
        logging: Some(LoggingCapability {}),
    })
    .for_websocket("0.0.0.0:7331")
    .await?;
```

---

## Cargo Features

Control what gets compiled into your binary:

```toml
[dependencies]
glyph = { version = "0.1", features = ["client", "server", "websocket"] }
```

**Available features:**

| Feature | Description | Default |
|---------|-------------|---------|
| `client` | MCP client library | ✅ |
| `server` | MCP server framework | ✅ |
| `websocket` | WebSocket transport | ✅ |
| `http` | HTTP/1.1 transport | ✅ |
| `http2` | HTTP/2 support (planned) | ❌ |
| `ffi` | FFI layer for Zig/C++ | ❌ |

**Minimal server:**
```toml
glyph = { version = "0.1", default-features = false, features = ["server"] }
```

**Client-only:**
```toml
glyph = { version = "0.1", default-features = false, features = ["client", "websocket"] }
```

---

## Built-in Tools

Glyph ships with 7 production-ready tools:

### 1. Echo Tool

**Name:** `echo`
**Description:** Echo back the input message

```json
{
  "name": "echo",
  "arguments": {
    "message": "Hello, World!"
  }
}
```

### 2. Read File

**Name:** `read_file`
**Description:** Read contents of a file

```json
{
  "name": "read_file",
  "arguments": {
    "path": "/path/to/file.txt"
  }
}
```

### 3. Write File

**Name:** `write_file`
**Description:** Write content to a file

```json
{
  "name": "write_file",
  "arguments": {
    "path": "/path/to/file.txt",
    "content": "file contents"
  }
}
```

### 4. List Directory

**Name:** `list_directory`
**Description:** List files in a directory

```json
{
  "name": "list_directory",
  "arguments": {
    "path": "/path/to/dir"
  }
}
```

### 5. Delete File

**Name:** `delete_file`
**Description:** Delete a file or directory

```json
{
  "name": "delete_file",
  "arguments": {
    "path": "/path/to/delete"
  }
}
```

### 6. Shell Execute

**Name:** `shell_execute`
**Description:** Execute a shell command

```json
{
  "name": "shell_execute",
  "arguments": {
    "command": "ls -la"
  }
}
```

⚠️ **Security:** Use with caution, requires consent mechanism

### 7. HTTP Request

**Name:** `http_request`
**Description:** Make HTTP requests

```json
{
  "name": "http_request",
  "arguments": {
    "url": "https://api.example.com/data",
    "method": "GET"
  }
}
```

---

## Advanced Features

### Policy Engine (Planned - Theta)

Implement consent gates:

```rust
async fn call(&self, ctx: &ToolContext, args: Option<Value>) -> Result<CallToolResult> {
    // Check permission
    ctx.guard.require("fs.write")?;

    // Proceed if granted
    tokio::fs::write(path, data).await?;
}
```

### Observability (Planned - Theta)

Structured logging and metrics:

```rust
use tracing::{info, instrument};

#[instrument(skip(self, ctx))]
async fn call(&self, ctx: &ToolContext, args: Option<Value>) -> Result<CallToolResult> {
    info!(tool = "my_tool", "Processing request");
    // ...
}
```

### Resource Subscriptions (Planned - Theta)

Real-time resource updates:

```rust
impl ResourceProvider for MyProvider {
    async fn subscribe(&self, uri: &str) -> Result<ResourceSubscription> {
        // Notify clients when resource changes
    }
}
```

---

## Performance Tips

### 1. Use Async I/O

```rust
// ✅ Good
let data = tokio::fs::read_to_string(path).await?;

// ❌ Bad (blocks async runtime)
let data = std::fs::read_to_string(path)?;
```

### 2. Batch Operations

```rust
// ✅ Good
let results = futures::future::join_all(tasks).await;

// ❌ Bad
for task in tasks {
    task.await;
}
```

### 3. Use Streaming for Large Data

```rust
use tokio_stream::StreamExt;

let mut stream = read_large_file();
while let Some(chunk) = stream.next().await {
    // Process chunk
}
```

### 4. Enable Release Profile Optimizations

```toml
[profile.release]
lto = true
codegen-units = 1
opt-level = 3
```

---

## Security Best Practices

### 1. Validate Input

```rust
async fn call(&self, _ctx: &ToolContext, args: Option<Value>) -> Result<CallToolResult> {
    let path = args["path"].as_str()
        .ok_or_else(|| Error::Protocol("Invalid path".into()))?;

    // Validate path
    if path.contains("..") {
        return Err(Error::Protocol("Path traversal detected".into()));
    }

    // ...
}
```

### 2. Use Tool Context for Auth

```rust
async fn call(&self, ctx: &ToolContext, args: Option<Value>) -> Result<CallToolResult> {
    // Check permissions
    ctx.guard.require("admin.delete")?;

    // Proceed only if authorized
}
```

### 3. Sanitize Outputs

```rust
fn sanitize_error(e: &str) -> String {
    e.replace("/home/user", "<redacted>")
}
```

---

## Testing Tools

### Unit Tests

```rust
#[tokio::test]
async fn test_my_tool() {
    let tool = MyTool;
    let ctx = ToolContext::new();
    let args = json!({"input": "test"});

    let result = tool.call(&ctx, Some(args)).await.unwrap();
    assert!(!result.is_error.unwrap_or(false));
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_server_integration() {
    let server = Server::builder()
        .for_websocket("127.0.0.1:0")
        .await
        .unwrap();

    let addr = server.local_addr().unwrap();
    tokio::spawn(server.run());

    let client = Client::connect_ws(&format!("ws://{}", addr)).await.unwrap();
    // Test tool calls
}
```

---

## Next Steps

- [Installation Guide](installation.md)
- [Quickstart](quickstart.md)
- [Integration Contract](INTEGRATION_CONTRACT.md)
- [Example Integration](../examples/integration_example/README.md)
