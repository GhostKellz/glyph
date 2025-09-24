# Transport Guide

Glyph supports multiple transport protocols for MCP communication, allowing flexibility in how clients and servers communicate. This guide covers the available transports, their configuration, and when to use each one.

## Available Transports

### 1. Standard I/O (stdio)

**Best for**: Local development, testing, and integration with command-line tools

**Description**: Uses standard input/output streams for MCP message passing. The server reads JSON-RPC messages from stdin and writes responses to stdout.

**Configuration**:
```bash
# Start server with stdio transport (default)
glyph serve --transport stdio

# Or explicitly specify
glyph serve --transport stdio
```

**Usage Example**:
```bash
# Pipe MCP messages to the server
echo '{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {...}}' | glyph serve
```

**Pros**:
- Simple and reliable
- No network configuration required
- Works in any environment
- Low resource overhead

**Cons**:
- Single client connection
- No concurrent requests
- Limited to local machine

### 2. WebSocket

**Best for**: Production deployments, web applications, and multi-client scenarios

**Description**: Uses WebSocket protocol for bidirectional, real-time communication over TCP. Supports multiple concurrent clients and provides connection management.

**Configuration**:
```bash
# Start server with WebSocket transport
glyph serve --transport websocket --port 3000

# With custom host
glyph serve --transport websocket --host 0.0.0.0 --port 8080

# With TLS (requires certificate files)
glyph serve --transport websocket --port 443 --tls-cert cert.pem --tls-key key.pem
```

**Client Connection**:
```javascript
// JavaScript WebSocket client example
const ws = new WebSocket('ws://localhost:3000');

ws.onopen = () => {
  // Send MCP initialize request
  ws.send(JSON.stringify({
    jsonrpc: '2.0',
    id: 1,
    method: 'initialize',
    params: {
      protocolVersion: '2024-11-05',
      capabilities: {},
      clientInfo: {
        name: 'example-client',
        version: '1.0.0'
      }
    }
  }));
};

ws.onmessage = (event) => {
  const response = JSON.parse(event.data);
  console.log('Received:', response);
};
```

**Pros**:
- Bidirectional communication
- Multiple concurrent clients
- Real-time messaging
- Web-native protocol
- TLS/SSL support

**Cons**:
- Requires network configuration
- Higher resource usage
- More complex setup

## Transport Selection

### Development vs Production

**Development**:
- Use `stdio` for simplicity
- Easy testing and debugging
- No network setup required

**Production**:
- Use `websocket` for scalability
- Supports multiple clients
- Better performance for concurrent requests

### Environment Considerations

**Local Development**:
```bash
glyph serve --transport stdio
```

**Docker Container**:
```bash
glyph serve --transport websocket --host 0.0.0.0 --port 3000
```

**Kubernetes Deployment**:
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: glyph-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: glyph
  template:
    metadata:
      labels:
        app: glyph
    spec:
      containers:
      - name: glyph
        image: glyph:latest
        command: ["glyph", "serve", "--transport", "websocket", "--port", "3000"]
        ports:
        - containerPort: 3000
```

**Serverless/Cloud Functions**:
- Use `stdio` transport
- Function runtime handles HTTP/WebSocket
- Glyph server becomes a function handler

## Transport Configuration

### Command Line Options

```bash
glyph serve --help
```

Common options:
- `--transport <TYPE>`: Transport type (stdio, websocket)
- `--host <HOST>`: Bind host (websocket only)
- `--port <PORT>`: Bind port (websocket only)
- `--tls-cert <FILE>`: TLS certificate file
- `--tls-key <FILE>`: TLS private key file

### Environment Variables

```bash
# WebSocket configuration
export GLYPH_WS_HOST=0.0.0.0
export GLYPH_WS_PORT=3000

# TLS configuration
export GLYPH_TLS_CERT=/path/to/cert.pem
export GLYPH_TLS_KEY=/path/to/key.pem

# Start server
glyph serve --transport websocket
```

### Configuration File

Create a `glyph.toml` configuration file:

```toml
[server]
transport = "websocket"

[websocket]
host = "0.0.0.0"
port = 3000
tls_cert = "/path/to/cert.pem"
tls_key = "/path/to/key.pem"
```

## Connection Management

### WebSocket Connection Lifecycle

1. **Connection Establishment**:
   - Client connects to WebSocket endpoint
   - Server accepts connection
   - MCP initialization handshake begins

2. **Message Handling**:
   - Bidirectional JSON-RPC message exchange
   - Server processes requests and sends responses
   - Client handles responses and notifications

3. **Connection Termination**:
   - Either side can close the connection
   - Clean shutdown with close frames
   - Automatic cleanup of resources

### Error Handling

**Connection Errors**:
- Network failures
- TLS certificate issues
- Invalid WebSocket handshake

**Message Errors**:
- Malformed JSON-RPC messages
- Protocol version mismatches
- Invalid method calls

**Recovery Strategies**:
- Automatic reconnection (client-side)
- Exponential backoff
- Connection pooling

## Security Considerations

### Network Security

**WebSocket Transport**:
- Use TLS (wss://) in production
- Implement proper certificate validation
- Configure firewall rules
- Use authentication/authorization

**stdio Transport**:
- Secure the execution environment
- Limit file system access
- Use process isolation

### Authentication

**WebSocket**:
- Implement token-based authentication
- Use secure WebSocket (WSS)
- Validate client certificates

**stdio**:
- Rely on OS-level security
- Use secure execution contexts

## Performance Tuning

### WebSocket Optimization

**Connection Limits**:
```bash
# Limit concurrent connections
glyph serve --transport websocket --max-connections 100
```

**Message Buffering**:
- Automatic message queuing
- Configurable buffer sizes
- Backpressure handling

**Resource Management**:
- Connection pooling
- Memory limits per connection
- Timeout configuration

### Monitoring

**Metrics**:
- Connection count
- Message throughput
- Error rates
- Latency measurements

**Logging**:
- Connection events
- Message processing
- Error conditions

## Troubleshooting

### Common Issues

**WebSocket Connection Refused**:
- Check if port is available
- Verify host binding
- Check firewall rules

**TLS Certificate Errors**:
- Validate certificate files
- Check certificate validity
- Verify certificate chain

**stdio Pipe Errors**:
- Check input/output redirection
- Verify JSON message format
- Check for process termination

### Debugging

**Enable Debug Logging**:
```bash
RUST_LOG=glyph=debug glyph serve --transport websocket
```

**Test Connections**:
```bash
# Test WebSocket connection
websocat ws://localhost:3000

# Test stdio with sample message
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | glyph serve
```

### Performance Monitoring

**Connection Stats**:
```bash
# Get server status
curl http://localhost:3000/status  # If status endpoint enabled
```

**Resource Usage**:
```bash
# Monitor process
top -p $(pgrep glyph)
```

## Advanced Configuration

### Custom Transport Implementation

Glyph's transport layer is extensible. You can implement custom transports by implementing the `TransportServer` trait:

```rust
use glyph::transport::TransportServer;
use async_trait::async_trait;

pub struct CustomTransport;

#[async_trait]
impl TransportServer for CustomTransport {
    async fn serve(&self, handler: RequestHandler) -> Result<(), Error> {
        // Custom transport implementation
        todo!()
    }
}
```

### Load Balancing

**Multiple Server Instances**:
- Run multiple Glyph servers
- Use load balancer (nginx, haproxy)
- Configure session affinity if needed

**Horizontal Scaling**:
- Stateless server design
- Shared storage for stateful operations
- Distributed tool execution

## Migration Guide

### From stdio to WebSocket

1. **Update Server Configuration**:
   ```bash
   # Before
   glyph serve --transport stdio

   # After
   glyph serve --transport websocket --port 3000
   ```

2. **Update Client Code**:
   ```javascript
   // Before: stdio pipe
   const child = spawn('glyph', ['serve']);

   // After: WebSocket connection
   const ws = new WebSocket('ws://localhost:3000');
   ```

3. **Test Migration**:
   - Verify all functionality works
   - Test error scenarios
   - Monitor performance

### Transport Compatibility

- MCP protocol is transport-agnostic
- Same tools and capabilities across transports
- Configuration differences only in connection setup

## Best Practices

### Production Deployment

1. **Use WebSocket with TLS**
2. **Implement proper logging**
3. **Set up monitoring and alerts**
4. **Configure resource limits**
5. **Use health checks**

### Development Workflow

1. **Start with stdio for simplicity**
2. **Use WebSocket for integration testing**
3. **Implement proper error handling**
4. **Test with realistic loads**

### Security First

1. **Always use TLS in production**
2. **Implement authentication**
3. **Validate all inputs**
4. **Monitor for anomalies**
5. **Keep dependencies updated**