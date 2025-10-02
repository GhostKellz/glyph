# Integration Contract

**Version**: 0.1.0
**Target integrations**: GhostLLM, GhostFlow, Jarvis, Rune (Zig), Zeke

This document describes the stable API surface and integration points for Glyph MCP, designed to support the GhostStack ecosystem.

---

## Table of Contents

1. [Overview](#overview)
2. [Core API Surface](#core-api-surface)
3. [Integration Patterns](#integration-patterns)
4. [Transport Protocols](#transport-protocols)
5. [Tool API](#tool-api)
6. [Resource API](#resource-api)
7. [Prompt API](#prompt-api)
8. [FFI Integration (Rune/Zig)](#ffi-integration-runezig)
9. [Authentication & Authorization](#authentication--authorization)
10. [Error Handling](#error-handling)
11. [Versioning & Compatibility](#versioning--compatibility)

---

## Overview

Glyph provides MCP server and client capabilities for the GhostStack ecosystem. Each integration has specific requirements:

### Integration Matrix

| Project | Role | Transport | Auth | Key Features |
|---------|------|-----------|------|--------------|
| **GhostLLM** | MCP Client | WebSocket/HTTP | API Key | Provider routing, cost tracking |
| **GhostFlow** | Both | WebSocket | Token | Flow execution, MCP node adapters |
| **Jarvis** | MCP Client | WebSocket/stdio | Interactive | CLI agent, consent prompts |
| **Rune** | FFI Consumer | C ABI | N/A | Zig MCP server/client |
| **Zeke** | MCP Server | stdio/WebSocket | Policy | Development tools, IDE integration |

---

## Core API Surface

### Rust Crate Interface

All integrations using Glyph as a Rust library depend on these stable modules:

```rust
// Core protocol types (JSON-RPC 2.0 + MCP)
use glyph::protocol::{
    // Requests & Responses
    CallToolRequest, CallToolResult,
    ListToolsRequest, ListToolsResponse,
    ReadResourceRequest, ReadResourceResponse,
    GetPromptRequest, GetPromptResponse,

    // Notifications
    ProgressNotification,
    LoggingNotification,

    // Types
    Tool, ToolInputSchema,
    Resource, ResourceContents,
    Prompt, PromptMessage,
    Content, TextContent, ImageContent,

    // Errors
    McpError, GlyphError,
};

// Server framework
use glyph::server::{
    Server, ServerBuilder,
    Tool as ToolTrait,
    ResourceProvider,
    PromptProvider,
    ToolContext,
};

// Client library
use glyph::client::{
    Client, ClientBuilder,
    ToolHandle, ResourceHandle, PromptHandle,
};

// Transports
use glyph::transport::{
    Transport, TransportConfig,
    StdioTransport, WebSocketTransport, HttpTransport,
};
```

### Stability Guarantees

- **Protocol types** (`glyph::protocol::*`): Stable across 0.1.x releases
- **Server/Client APIs**: Additive changes only in 0.1.x
- **Transport interfaces**: Stable trait definitions
- **FFI surface**: ABI-stable within major version

---

## Integration Patterns

### Pattern 1: GhostLLM - Provider Passthrough

**Use case**: Expose OpenAI/Anthropic/Gemini as MCP tools

```rust
use glyph::server::{Server, Tool, ToolContext};
use serde_json::{json, Value};

struct OpenAITool {
    api_key: String,
}

#[async_trait::async_trait]
impl Tool for OpenAITool {
    fn name(&self) -> &str { "openai_chat" }

    async fn call(&self, ctx: &ToolContext, input: Value) -> glyph::Result<CallToolResult> {
        // GhostLLM routing logic
        ctx.guard.require("llm.invoke")?;

        // Track cost metadata
        let meta = json!({
            "provider": "openai",
            "model": input["model"],
            "cost_usd": calculate_cost(&input),
        });

        let response = call_openai(&self.api_key, input).await?;

        Ok(CallToolResult {
            content: vec![Content::text(response)],
            is_error: None,
            meta: Some(meta),
        })
    }
}
```

**Integration requirements**:
- Authentication: API key or token in tool context
- Cost tracking: Populate `meta` field in `CallToolResult`
- Rate limiting: Implement in tool `call()` method

---

### Pattern 2: GhostFlow - MCP Node Adapter

**Use case**: Call Glyph MCP tools from n8n-style workflow nodes

```rust
use glyph::client::Client;

pub struct McpNode {
    client: Client,
}

impl McpNode {
    pub async fn execute(&self, tool: &str, params: Value) -> Result<NodeOutput> {
        let result = self.client
            .tool(tool)
            .invoke(params)
            .await?;

        Ok(NodeOutput {
            data: result.content,
            metadata: result.meta,
        })
    }
}
```

**Integration requirements**:
- Client mode: Use `glyph::client::Client`
- Async execution: All MCP calls are async
- Error mapping: Convert `GlyphError` to GhostFlow errors

---

### Pattern 3: Jarvis - CLI Agent with Consent

**Use case**: Local AI agent with user approval for sensitive operations

```rust
use glyph::server::{Server, Tool, ToolContext};

#[async_trait::async_trait]
impl Tool for ShellTool {
    async fn call(&self, ctx: &ToolContext, input: Value) -> glyph::Result<CallToolResult> {
        // Jarvis consent mechanism
        let command = input["command"].as_str().unwrap();

        if ctx.guard.require("shell.execute").is_err() {
            return Ok(CallToolResult {
                content: vec![Content::text("Permission denied by user")],
                is_error: Some(true),
                meta: None,
            });
        }

        // Execute after consent
        let output = execute_shell(command).await?;
        Ok(CallToolResult {
            content: vec![Content::text(output)],
            is_error: None,
            meta: None,
        })
    }
}
```

**Integration requirements**:
- Policy engine: Use `ToolContext::guard` for consent
- Interactive prompts: Hook into `guard.require()` failures
- Audit logging: Log all tool invocations

---

## Transport Protocols

### stdio (for Jarvis, Zeke)

**Protocol**: JSON-RPC 2.0 over stdin/stdout
**Format**: One JSON object per line (newline-delimited)

```bash
# Server side
glyph serve --transport stdio

# Client side (Jarvis/Zeke)
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"echo","arguments":{}}}' | glyph serve --transport stdio
```

**Connection lifecycle**:
1. Client sends `initialize` request
2. Server responds with capabilities
3. Client sends `initialized` notification
4. Tool/resource requests begin

---

### WebSocket (for GhostLLM, GhostFlow)

**Protocol**: JSON-RPC 2.0 over WebSocket
**URL**: `ws://host:port` or `wss://host:port`

```rust
// Server
let server = Server::builder()
    .transport_websocket("127.0.0.1:7331")
    .build()
    .await?;

// Client
let client = Client::connect_ws("ws://127.0.0.1:7331").await?;
```

**Message format**:
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "tool_name",
    "arguments": { "key": "value" }
  }
}
```

---

## Tool API

### Implementing a Tool

```rust
use glyph::server::{Tool, ToolContext};
use glyph::protocol::{ToolInputSchema, CallToolResult, Content};
use serde_json::Value;
use std::collections::HashMap;

struct CustomTool;

#[async_trait::async_trait]
impl Tool for CustomTool {
    fn name(&self) -> &str {
        "custom_tool"
    }

    fn description(&self) -> Option<&str> {
        Some("Description for AI models")
    }

    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema {
            schema_type: "object".to_string(),
            properties: Some({
                let mut props = HashMap::new();
                props.insert("input".to_string(), serde_json::json!({
                    "type": "string",
                    "description": "Input parameter"
                }));
                props
            }),
            required: Some(vec!["input".to_string()]),
            additional: HashMap::new(),
        }
    }

    async fn call(&self, ctx: &ToolContext, args: Option<Value>) -> glyph::Result<CallToolResult> {
        let input = args.ok_or_else(|| glyph::Error::Protocol("Missing args".into()))?;

        // Business logic
        let result = process(input)?;

        Ok(CallToolResult {
            content: vec![Content::text(result)],
            is_error: None,
            meta: None,
        })
    }
}
```

### Registering Tools

```rust
let mut server = Server::builder()
    .transport_stdio()
    .build()
    .await?;

server.register(CustomTool);
server.run().await?;
```

---

## Resource API

Resources provide read-only access to data (files, databases, APIs).

```rust
use glyph::server::ResourceProvider;
use glyph::protocol::{Resource, ResourceContents, TextResourceContents};

struct FileResourceProvider;

#[async_trait::async_trait]
impl ResourceProvider for FileResourceProvider {
    async fn list(&self) -> glyph::Result<Vec<Resource>> {
        Ok(vec![Resource {
            uri: "file:///etc/hosts".to_string(),
            name: "System Hosts File".to_string(),
            description: Some("OS hostname mappings".to_string()),
            mime_type: Some("text/plain".to_string()),
        }])
    }

    async fn read(&self, uri: &str) -> glyph::Result<ResourceContents> {
        let data = tokio::fs::read_to_string(uri).await?;
        Ok(ResourceContents::Text(TextResourceContents {
            uri: uri.to_string(),
            mime_type: Some("text/plain".to_string()),
            text: data,
        }))
    }
}
```

---

## Prompt API

Prompts are reusable templates with variable substitution.

```rust
use glyph::server::PromptProvider;
use glyph::protocol::{Prompt, PromptMessage, Role};

struct CodeReviewPrompt;

#[async_trait::async_trait]
impl PromptProvider for CodeReviewPrompt {
    fn name(&self) -> &str {
        "code_review"
    }

    async fn get(&self, args: Option<Value>) -> glyph::Result<Prompt> {
        let code = args.and_then(|v| v["code"].as_str()).unwrap_or("");

        Ok(Prompt {
            messages: vec![
                PromptMessage {
                    role: Role::System,
                    content: Content::text("You are a code reviewer."),
                },
                PromptMessage {
                    role: Role::User,
                    content: Content::text(format!("Review this code:\n{}", code)),
                },
            ],
            description: Some("Code review assistant".to_string()),
        })
    }
}
```

---

## FFI Integration (Rune/Zig)

Glyph exposes a C ABI for Zig integration via **Rune**.

### C Header (auto-generated)

```c
// glyph.h

typedef struct GlyphServer GlyphServer;
typedef struct GlyphClient GlyphClient;

// Server functions
GlyphServer* glyph_server_new_stdio(void);
GlyphServer* glyph_server_new_ws(const char* addr);
int glyph_server_register_tool(GlyphServer* server, const char* name, ToolCallback cb);
int glyph_server_run(GlyphServer* server);
void glyph_server_free(GlyphServer* server);

// Client functions
GlyphClient* glyph_client_connect_ws(const char* url);
char* glyph_client_call_tool(GlyphClient* client, const char* name, const char* args_json);
void glyph_client_free(GlyphClient* client);
```

### Zig Usage (via Rune)

```zig
const rune = @import("rune");

pub fn main() !void {
    var client = try rune.Client.connectWs(alloc, "ws://localhost:7331");
    defer client.deinit();

    const result = try client.invoke(.{
        .tool = "read_file",
        .input = .{ .path = "/etc/hosts" },
    });

    std.debug.print("{s}\n", .{result.text});
}
```

**FFI Contract**:
- Strings: UTF-8 null-terminated `const char*`
- Memory: Caller owns returned strings (must free)
- Errors: Non-zero return codes indicate failure
- Threading: All calls are thread-safe

---

## Authentication & Authorization

### API Key Authentication

```rust
use glyph::server::middleware::ApiKeyAuth;

let server = Server::builder()
    .middleware(ApiKeyAuth::new("your-secret-key"))
    .transport_websocket("0.0.0.0:7331")
    .build()
    .await?;
```

### Policy Gates (ToolContext::guard)

```rust
async fn call(&self, ctx: &ToolContext, input: Value) -> Result<CallToolResult> {
    // Check permission before execution
    ctx.guard.require("fs.write")?;

    // Proceed if granted
    tokio::fs::write(path, data).await?;
    Ok(...)
}
```

**Integration expectations**:
- **GhostLLM**: API key middleware + cost tracking
- **GhostFlow**: Token-based auth for multi-tenant
- **Jarvis**: Interactive consent UI on `guard.require()` failure
- **Zeke**: Policy config file (`~/.zeke/policy.toml`)

---

## Error Handling

### Error Types

```rust
pub enum GlyphError {
    Protocol(String),      // MCP protocol violations
    Transport(String),     // Network/IO errors
    JsonRpc(String),       // JSON-RPC errors
    Internal(String),      // Internal bugs
}

pub struct McpError {
    pub code: i32,         // JSON-RPC error code
    pub message: String,
    pub data: Option<Value>,
}
```

### Error Codes (JSON-RPC)

| Code | Meaning | Usage |
|------|---------|-------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid request | Missing required fields |
| -32601 | Method not found | Unknown MCP method |
| -32602 | Invalid params | Bad tool arguments |
| -32603 | Internal error | Server crash |
| -32000 | Tool error | Tool execution failed |

### Error Response Format

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "error": {
    "code": -32000,
    "message": "Tool execution failed",
    "data": {
      "tool": "shell_execute",
      "reason": "Command not found"
    }
  }
}
```

---

## Versioning & Compatibility

### Protocol Version

Glyph implements **MCP version 2024-11-05**.

### Library Versioning

- **0.1.x**: Alpha/Beta releases, additive changes only
- **0.2.0**: First breaking change (stable API established)
- **1.0.0**: GA release, SemVer guarantees begin

### Migration Paths

When integrating:
1. Pin to specific tag: `glyph = { git = "...", tag = "v0.1.0" }`
2. Review `CHANGELOG.md` before upgrading
3. Run integration tests after version bumps

---

## Testing Integrations

### Smoke Test Suite

Each integration should run these tests:

1. **Connection**: Can connect to Glyph server
2. **Initialize**: Handshake completes successfully
3. **List tools**: Can enumerate available tools
4. **Call tool**: Can invoke `echo` tool
5. **Error handling**: Gracefully handles invalid requests

### Example Test (Rust)

```rust
#[tokio::test]
async fn test_ghostllm_integration() {
    let server = Server::builder()
        .transport_websocket("127.0.0.1:0")
        .build()
        .await
        .unwrap();

    let addr = server.local_addr().unwrap();

    tokio::spawn(server.run());

    let client = Client::connect_ws(&format!("ws://{}", addr))
        .await
        .unwrap();

    let tools = client.list_tools().await.unwrap();
    assert!(!tools.is_empty());
}
```

---

## Support

For integration questions:
- **Issues**: https://github.com/ghostkellz/glyph/issues
- **Discussions**: https://github.com/ghostkellz/glyph/discussions
- **Examples**: See `examples/` directory in repo

---

**Document Version**: 0.1.0
**Last Updated**: 2025-10-02
**Maintained By**: GhostStack Team
