# Architecture Overview

This document provides a comprehensive overview of Glyph's architecture, design principles, and component interactions.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                          Application Layer                       │
├─────────────────────────────────────────────────────────────────┤
│                     Glyph Client/Server API                     │
├─────────────────────────┬───────────────────────────────────────┤
│        Client           │            Server                     │
│                         │                                       │
│  ┌─────────────────┐   │   ┌─────────────────────────────────┐ │
│  │ Tool Client     │   │   │ Request Handler                 │ │
│  │ Resource Client │   │   │ ┌─────────────┬─────────────────┤ │
│  │ Prompt Client   │   │   │ │Tool Registry│Resource Registry│ │
│  └─────────────────┘   │   │ │             │Prompt Registry  │ │
│                         │   │ └─────────────┴─────────────────┤ │
│  ┌─────────────────┐   │   │ Session Manager                 │ │
│  │ Connection      │   │   └─────────────────────────────────┘ │
│  └─────────────────┘   │                                       │
├─────────────────────────┼───────────────────────────────────────┤
│                    Transport Layer                              │
│  ┌─────────────────────────────────────────────────────────────┤
│  │ ┌─────────────┐ ┌─────────────┐ ┌─────────────────────────┐ │
│  │ │   stdio     │ │ WebSocket   │ │       HTTP/SSE         │ │
│  │ └─────────────┘ └─────────────┘ └─────────────────────────┘ │
│  └─────────────────────────────────────────────────────────────┤
├─────────────────────────────────────────────────────────────────┤
│                      Protocol Layer                            │
│  ┌─────────────────────────────────────────────────────────────┤
│  │ JSON-RPC 2.0 │ MCP Messages │ Error Handling │ Capabilities │
│  └─────────────────────────────────────────────────────────────┤
└─────────────────────────────────────────────────────────────────┘
```

## Core Design Principles

### 1. Type Safety
- Strong typing throughout with comprehensive error handling
- Serde-based serialization with proper error propagation
- Protocol version negotiation to ensure compatibility

### 2. Async-First
- Built on Tokio for high-performance async I/O
- Non-blocking operations throughout the stack
- Efficient connection pooling and resource management

### 3. Transport Abstraction
- Pluggable transport layer supporting multiple protocols
- Consistent API regardless of underlying transport
- Easy to add new transport implementations

### 4. Modular Design
- Clear separation between protocol, transport, server, and client
- Composable components that can be used independently
- Extensible architecture for custom tools and resources

### 5. Enterprise Ready
- Authentication and authorization hooks
- Comprehensive audit logging and observability
- Graceful error handling and recovery

## Component Deep Dive

### Protocol Layer (`src/protocol/`)

The protocol layer implements the core MCP specification:

#### Types (`types.rs`)
- Core data structures (RequestId, Implementation, Content, etc.)
- JSON Schema definitions for tools and resources
- Serialization/deserialization with proper validation

#### JSON-RPC (`jsonrpc.rs`)
- JSON-RPC 2.0 message envelope handling
- Request/response correlation
- Notification support for real-time updates

#### Messages (`messages.rs`)
- All MCP protocol messages (Initialize, ListTools, CallTool, etc.)
- Proper parameter validation and error responses
- Support for pagination and streaming

#### Capabilities (`capabilities.rs`)
- Client and server capability negotiation
- Feature detection and graceful degradation
- Extensible capability system

#### Errors (`error.rs`)
- Comprehensive error types covering all failure modes
- Proper error codes following MCP specification
- Rich error context for debugging

### Transport Layer (`src/transport/`)

Provides multiple transport implementations with a unified interface:

#### Transport Trait (`traits.rs`)
```rust
#[async_trait]
pub trait Transport: Send + Sync + Debug {
    async fn send(&mut self, message: JsonRpcMessage) -> Result<()>;
    async fn receive(&mut self) -> Result<Option<JsonRpcMessage>>;
    async fn close(&mut self) -> Result<()>;
    fn is_closed(&self) -> bool;
}
```

#### stdio Transport (`stdio.rs`)
- Standard input/output communication
- Line-based JSON message exchange
- Ideal for CLI tools and subprocess communication

#### WebSocket Transport (`websocket.rs`)
- Real-time bidirectional communication
- Automatic ping/pong handling
- Suitable for interactive applications

#### HTTP Transport (`http.rs`)
- HTTP POST for requests
- Server-Sent Events (SSE) for real-time updates
- Good for web applications and REST APIs

### Server Framework (`src/server/`)

#### Core Server (`core.rs`)
The main server implementation handles:
- Connection lifecycle management
- Request routing and processing
- Session management
- Graceful shutdown

#### Tool System (`tools.rs`)
```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> Option<&str>;
    fn input_schema(&self) -> ToolInputSchema;
    async fn call(&self, args: Option<Value>) -> Result<CallToolResult>;
}
```

Built-in tools include:
- **EchoTool**: Simple echo for testing
- **ReadFileTool**: Read file contents
- **WriteFileTool**: Write data to files

#### Resource System (`resources.rs`)
```rust
#[async_trait]
pub trait ResourceProvider: Send + Sync {
    async fn list_resources(&self) -> Result<Vec<Resource>>;
    async fn read_resource(&self, uri: &str) -> Result<Vec<ResourceContents>>;
    async fn subscribe(&self, uri: &str) -> Result<()>;
    async fn unsubscribe(&self, uri: &str) -> Result<()>;
}
```

Built-in providers:
- **FileSystemResourceProvider**: Expose filesystem as resources
- **MemoryResourceProvider**: In-memory resource storage

#### Prompt System (`prompts.rs`)
```rust
#[async_trait]
pub trait PromptProvider: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> Option<&str>;
    fn arguments(&self) -> Vec<PromptArgument>;
    async fn get_prompt(&self, arguments: HashMap<String, String>) -> Result<GetPromptResult>;
}
```

### Client Library (`src/client/`)

#### Core Client (`core.rs`)
- Connection management and message routing
- Request/response correlation with timeouts
- Automatic reconnection and error recovery

#### Specialized Clients
- **ToolClient**: Tool discovery and execution
- **ResourceClient**: Resource access and subscription
- **PromptClient**: Prompt rendering and execution

## Message Flow

### Client Request Flow

```
Client Application
       ↓
   ToolClient.call_tool()
       ↓
   Connection.send()
       ↓
   Transport.send()
       ↓
   [Network/Process Boundary]
       ↓
   Transport.receive()
       ↓
   Server.handle_message()
       ↓
   RequestHandler.handle_request()
       ↓
   ToolRegistry.call_tool()
       ↓
   Tool.call()
       ↓
   [Return path with response]
```

### Server Initialization Flow

```
Server.builder()
    .with_tools()
    .build()
       ↓
Server.register_tool()
       ↓
Server.run_with_transport()
       ↓
Transport.receive() [Loop]
       ↓
Server.handle_message()
       ↓
  [Route to appropriate handler]
```

## Concurrency Model

### Server Concurrency
- **Multi-threaded**: Each client connection runs in its own task
- **Shared State**: Registries are wrapped in Arc<RwLock<>> for thread safety
- **Connection Isolation**: Client failures don't affect other connections

### Client Concurrency
- **Request Correlation**: Multiple concurrent requests with proper ID tracking
- **Response Handling**: Async channel-based response waiting
- **Resource Sharing**: Connection pooling for efficient resource usage

## Error Handling Strategy

### Error Types Hierarchy
```
GlyphError
├── Mcp(McpError)           # Protocol-level errors
├── JsonRpc(String)         # JSON-RPC specific errors
├── Transport(String)       # Transport-layer errors
├── Serialization(serde_json::Error) # Serialization failures
├── Io(std::io::Error)      # I/O errors
├── ConnectionClosed        # Connection state errors
└── Timeout                 # Operation timeouts
```

### Error Recovery
- **Graceful Degradation**: Continue operation when possible
- **Retry Logic**: Automatic retry with exponential backoff
- **Circuit Breaker**: Prevent cascade failures
- **Detailed Logging**: Comprehensive error context for debugging

## Performance Characteristics

### Throughput
- **Design Target**: >10,000 concurrent connections
- **Memory Usage**: <100MB per 1000 clients
- **Latency**: <10ms P99 for local tool calls

### Optimization Strategies
- **Zero-Copy**: Minimize data copying in hot paths
- **Connection Pooling**: Reuse transport connections
- **Batch Processing**: Group related operations
- **Lazy Loading**: Load resources on demand

## Security Model

### Authentication
- Pluggable authentication providers
- Token-based authentication (JWT, API keys)
- Client certificate validation

### Authorization
- Role-based access control (RBAC)
- Tool-level permissions
- Resource access controls

### Audit Logging
- Comprehensive request/response logging
- Structured log format for analysis
- Sensitive data redaction

## Extension Points

### Custom Tools
Implement the `Tool` trait to add functionality:
```rust
struct MyCustomTool;

#[async_trait]
impl Tool for MyCustomTool {
    fn name(&self) -> &str { "my_tool" }
    async fn call(&self, args: Option<Value>) -> Result<CallToolResult> {
        // Custom implementation
    }
}
```

### Custom Transports
Implement the `Transport` trait for new protocols:
```rust
struct MyTransport;

#[async_trait]
impl Transport for MyTransport {
    async fn send(&mut self, message: JsonRpcMessage) -> Result<()> {
        // Custom transport implementation
    }
    // ... other methods
}
```

### Middleware
Add cross-cutting concerns:
```rust
struct AuthMiddleware;

#[async_trait]
impl Middleware for AuthMiddleware {
    async fn before_request(&self, request: &mut JsonRpcRequest<Value>) -> Result<()> {
        // Authentication logic
    }
}
```

## Integration Architecture

### GhostLLM Integration
```
GhostLLM (Zig)
      ↓ FFI
Glyph (Rust) ← MCP Protocol → AI Clients
      ↓
Native Tools & Resources
```

### GhostFlow Integration
```
Flow Nodes ← MCP Tools → Glyph Server
                ↓
         External Services
```

### Deployment Patterns

#### Embedded Server
```rust
// Embed server in existing application
let server = Server::builder().build();
tokio::spawn(async move {
    server.run_with_transport(transport).await
});
```

#### Standalone Service
```rust
// Run as microservice
#[tokio::main]
async fn main() {
    let server = Server::builder()
        .for_websocket("0.0.0.0:8080")
        .await?;
    server.run().await
}
```

#### Library Integration
```rust
// Use as library in larger application
let client = Client::connect_websocket(url).await?;
let result = client.tools().call_tool("analyze", Some(data)).await?;
```

This architecture enables Glyph to serve as both a standalone MCP implementation and as a foundational library for building sophisticated AI-powered applications.