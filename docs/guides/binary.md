# Binary Usage Guide

The Glyph binary provides a ready-to-use MCP server with 7 built-in tools. This guide covers installation, configuration, and usage.

## Installation

### From Source (Recommended)

```bash
git clone https://github.com/ghostkellz/glyph
cd glyph
cargo build --release
```

The binary will be at `target/release/glyph`.

### From Crates.io (Future)

```bash
cargo install glyph
```

## Basic Usage

### Start WebSocket Server

```bash
./glyph serve
```

This starts the server on `ws://127.0.0.1:7331` with all built-in tools enabled.

### Command Line Options

```bash
# Show help
./glyph --help

# Show serve command help
./glyph serve --help
```

### Configuration Options

```bash
# Custom address and port
./glyph serve --address 0.0.0.0:8080

# Enable verbose logging
./glyph serve --verbose

# Use stdio transport instead of WebSocket
./glyph serve --transport stdio
```

## Built-in Tools

The binary includes 7 production-ready tools:

### 1. echo
**Purpose**: Echo back input messages
**Parameters**:
- `message` (string): The message to echo

### 2. read_file
**Purpose**: Read file contents
**Parameters**:
- `path` (string): Path to the file to read

### 3. write_file
**Purpose**: Write content to files
**Parameters**:
- `path` (string): Path to the file to write
- `content` (string): Content to write

### 4. list_directory
**Purpose**: List directory contents
**Parameters**:
- `path` (string): Directory path to list

### 5. delete_file
**Purpose**: Delete files or empty directories
**Parameters**:
- `path` (string): Path to delete

### 6. shell_execute
**Purpose**: Execute shell commands
**Parameters**:
- `command` (string): Command to execute
- `timeout` (number, optional): Timeout in seconds
- `working_directory` (string, optional): Working directory

### 7. http_request
**Purpose**: Make HTTP requests to external APIs
**Parameters**:
- `url` (string): URL to request
- `method` (string, optional): HTTP method (default: GET)
- `headers` (object, optional): HTTP headers
- `body` (string, optional): Request body

## Testing the Server

### WebSocket Testing

```bash
# Start server in background
./glyph serve &

# Test with the provided test client
cargo run --example test_client
```

### Stdio Testing

```bash
# Test stdio transport
echo '{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "clientInfo": {"name": "test", "version": "1.0"}}}' | ./glyph serve --transport stdio
```

### Manual WebSocket Testing

```bash
# Connect with websocat (if installed)
websocat ws://127.0.0.1:7331

# Send initialize message
{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2024-11-05", "capabilities": {"tools": {}}, "clientInfo": {"name": "test", "version": "1.0"}}}
```

## MCP Client Integration

### Claude Desktop

Add to your Claude Desktop configuration:

```json
{
  "mcpServers": {
    "glyph": {
      "command": "/path/to/glyph",
      "args": ["serve"],
      "env": {}
    }
  }
}
```

### Other MCP Clients

Most MCP clients support WebSocket connections. Configure them to connect to:
- **URL**: `ws://127.0.0.1:7331`
- **Protocol**: MCP over WebSocket

## Production Deployment

### Docker

```dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/glyph /usr/local/bin/glyph
EXPOSE 7331
CMD ["glyph", "serve", "--address", "0.0.0.0:7331"]
```

```bash
docker build -t glyph-mcp .
docker run -p 7331:7331 glyph-mcp
```

### Systemd Service

Create `/etc/systemd/system/glyph-mcp.service`:

```ini
[Unit]
Description=Glyph MCP Server
After=network.target

[Service]
Type=simple
User=glyph
ExecStart=/usr/local/bin/glyph serve --address 0.0.0.0:7331
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable glyph-mcp
sudo systemctl start glyph-mcp
```

## Security Considerations

### Network Access
- By default, the server binds to `127.0.0.1` (localhost only)
- For network access, use `--address 0.0.0.0:7331`
- Consider firewall rules and authentication for production

### File System Access
- Tools can read/write files and execute commands
- Run as unprivileged user in production
- Use containerization or chroot for isolation

### Logging
- Use `--verbose` for debugging
- Logs include MCP protocol details
- Consider log rotation for production

## Troubleshooting

### Server won't start
```bash
# Check if port is in use
netstat -tlnp | grep 7331

# Try different port
./glyph serve --address 127.0.0.1:8080
```

### Client can't connect
```bash
# Test basic connectivity
curl -I http://127.0.0.1:7331

# Check server logs
./glyph serve --verbose
```

### Tools not working
```bash
# Test with verbose logging
./glyph serve --verbose

# Check file permissions
ls -la /path/to/test/file

# Test shell commands manually
/path/to/command
```

## Performance Tuning

### Connection Limits
- Default: No explicit limits
- Consider reverse proxy (nginx) for production
- Monitor memory usage with many connections

### Tool Timeouts
- Shell commands have default timeouts
- Configure based on your use case
- Long-running commands may need special handling

## Next Steps

- [Server Guide](../guides/server.md) - Build custom servers
- [Built-in Tools](../guides/tools.md) - Detailed tool documentation
- [Transport Guide](../guides/transports.md) - Advanced transport options