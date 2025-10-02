# Glyph Integration Example

A complete example showing how to integrate Glyph MCP into your Rust application.

## What This Demonstrates

- **Custom Tools**: Creating tools with typed input schemas and validation
- **Custom Resources**: Providing read-only access to application data
- **Custom Prompts**: Building reusable prompt templates
- **Multiple Transports**: Supporting both stdio and WebSocket
- **Proper Error Handling**: Using Glyph's error types
- **Logging**: Structured logging with tracing

## Project Structure

```
integration_example/
├── Cargo.toml          # Dependencies
├── src/
│   └── main.rs         # Main application
└── README.md           # This file
```

## Building

```bash
cd examples/integration_example
cargo build --release
```

## Running

### Stdio Transport (for CLI clients)

```bash
cargo run

# Or with explicit transport:
TRANSPORT=stdio cargo run
```

### WebSocket Transport (for web clients)

```bash
TRANSPORT=websocket ADDRESS=127.0.0.1:7331 cargo run
```

## Testing with Client

Use the Glyph test client to interact with the server:

```bash
# In another terminal
cd ../..
cargo run --example test_client
```

Or connect manually using JSON-RPC:

### Initialize Session

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {},
    "clientInfo": {
      "name": "test-client",
      "version": "1.0.0"
    }
  }
}
```

### List Tools

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list",
  "params": {}
}
```

Response:
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "tools": [
      {
        "name": "calculator",
        "description": "Perform basic arithmetic calculations",
        "inputSchema": {
          "type": "object",
          "properties": {
            "operation": {
              "type": "string",
              "enum": ["add", "subtract", "multiply", "divide"]
            },
            "a": { "type": "number" },
            "b": { "type": "number" }
          },
          "required": ["operation", "a", "b"]
        }
      }
    ]
  }
}
```

### Call Calculator Tool

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "calculator",
    "arguments": {
      "operation": "multiply",
      "a": 7,
      "b": 6
    }
  }
}
```

Response:
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "7 multiply 6 = 42"
      }
    ],
    "_meta": {
      "operation": "multiply",
      "result": 42
    }
  }
}
```

### List Resources

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "resources/list",
  "params": {}
}
```

### Read Resource

```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "resources/read",
  "params": {
    "uri": "config://app.json"
  }
}
```

### Get Prompt

```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "prompts/get",
  "params": {
    "name": "code_review",
    "arguments": {
      "code": "fn main() { println!(\"Hello\"); }",
      "language": "rust"
    }
  }
}
```

## Extending This Example

### Add Your Own Tool

```rust
struct MyCustomTool;

#[async_trait]
impl Tool for MyCustomTool {
    fn name(&self) -> &str {
        "my_tool"
    }

    async fn call(&self, _ctx: &ToolContext, args: Option<Value>) -> Result<CallToolResult> {
        // Your implementation
        Ok(CallToolResult {
            content: vec![Content::text("result")],
            is_error: None,
            meta: None,
        })
    }

    fn input_schema(&self) -> ToolInputSchema {
        // Define your schema
        ToolInputSchema::default()
    }
}

// Register it:
server.register_tool(Box::new(MyCustomTool)).await?;
```

### Add Database Access

```rust
use sqlx::PgPool;

struct DatabaseTool {
    pool: PgPool,
}

#[async_trait]
impl Tool for DatabaseTool {
    fn name(&self) -> &str {
        "query_db"
    }

    async fn call(&self, _ctx: &ToolContext, args: Option<Value>) -> Result<CallToolResult> {
        let query = args["query"].as_str().unwrap();
        let rows = sqlx::query(query).fetch_all(&self.pool).await?;
        // ... format results
    }
}
```

### Add Authentication

```rust
use glyph::server::middleware::ApiKeyAuth;

let server = Server::builder()
    .middleware(ApiKeyAuth::new("your-secret-key"))
    .for_websocket("0.0.0.0:7331")
    .await?;
```

## Integration Patterns

This example follows the patterns described in [INTEGRATION_CONTRACT.md](../../docs/INTEGRATION_CONTRACT.md).

See also:
- [Glyph Documentation](../../docs/README.md)
- [Installation Guide](../../docs/installation.md)
- [Tool API Reference](../../docs/INTEGRATION_CONTRACT.md#tool-api)

## License

MIT
