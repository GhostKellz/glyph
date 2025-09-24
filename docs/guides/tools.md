# Built-in Tools Guide

The Glyph binary comes with 7 production-ready tools that provide common functionality out of the box. This guide documents each tool's capabilities, parameters, and usage examples.

## Available Tools

### 1. echo

**Purpose**: Echo back input messages for testing and debugging.

**Parameters**:
- `message` (string, required): The message to echo back

**Example**:
```json
{
  "name": "echo",
  "arguments": {
    "message": "Hello, World!"
  }
}
```

**Response**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "Hello, World!"
    }
  ]
}
```

### 2. read_file

**Purpose**: Read the contents of a file from the file system.

**Parameters**:
- `path` (string, required): Absolute or relative path to the file

**Example**:
```json
{
  "name": "read_file",
  "arguments": {
    "path": "/etc/hosts"
  }
}
```

**Response**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "127.0.0.1 localhost\n::1 localhost\n"
    }
  ]
}
```

**Error Cases**:
- File doesn't exist
- Permission denied
- Path is a directory

### 3. write_file

**Purpose**: Write content to a file, creating it if it doesn't exist.

**Parameters**:
- `path` (string, required): Path where to write the file
- `content` (string, required): Content to write to the file

**Example**:
```json
{
  "name": "write_file",
  "arguments": {
    "path": "/tmp/test.txt",
    "content": "Hello, World!\nThis is a test file."
  }
}
```

**Response**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "File written successfully"
    }
  ]
}
```

**Notes**:
- Creates parent directories if they don't exist
- Overwrites existing files
- Content is written as UTF-8

### 4. list_directory

**Purpose**: List the contents of a directory.

**Parameters**:
- `path` (string, required): Path to the directory to list

**Example**:
```json
{
  "name": "list_directory",
  "arguments": {
    "path": "/tmp"
  }
}
```

**Response**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "Contents of /tmp:\n- file1.txt (file, 1024 bytes)\n- subdir/ (directory)\n- script.sh (file, executable, 512 bytes)"
    }
  ]
}
```

**Features**:
- Shows file types (file/directory)
- Shows file sizes
- Indicates executable permissions
- Sorted alphabetically

### 5. delete_file

**Purpose**: Delete a file or empty directory.

**Parameters**:
- `path` (string, required): Path to the file or directory to delete

**Example**:
```json
{
  "name": "delete_file",
  "arguments": {
    "path": "/tmp/test.txt"
  }
}
```

**Response**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "File deleted successfully"
    }
  ]
}
```

**Notes**:
- Only deletes empty directories
- Returns error for non-empty directories
- No confirmation prompt (use with caution)

### 6. shell_execute

**Purpose**: Execute shell commands with optional timeout and working directory.

**Parameters**:
- `command` (string, required): Shell command to execute
- `timeout` (number, optional): Timeout in seconds (default: 30)
- `working_directory` (string, optional): Working directory for command execution

**Example**:
```json
{
  "name": "shell_execute",
  "arguments": {
    "command": "ls -la",
    "working_directory": "/tmp",
    "timeout": 10
  }
}
```

**Response**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "STDOUT:\n total 8\ndrwxrwxrwt 2 root root 4096 Jan 1 12:00 .\ndrwxr-xr-x 18 root root 4096 Jan 1 12:00 ..\n\nExit code: 0"
    }
  ]
}
```

**Features**:
- Captures both stdout and stderr
- Shows exit code
- Timeout protection
- Working directory support
- Safe command execution

### 7. http_request

**Purpose**: Make HTTP requests to external APIs and web services.

**Parameters**:
- `url` (string, required): URL to request
- `method` (string, optional): HTTP method (default: "GET")
- `headers` (object, optional): HTTP headers as key-value pairs
- `body` (string, optional): Request body for POST/PUT requests

**Examples**:

**GET Request**:
```json
{
  "name": "http_request",
  "arguments": {
    "url": "https://api.github.com/user",
    "headers": {
      "Authorization": "Bearer YOUR_TOKEN",
      "User-Agent": "Glyph-MCP/1.0"
    }
  }
}
```

**POST Request**:
```json
{
  "name": "http_request",
  "arguments": {
    "url": "https://httpbin.org/post",
    "method": "POST",
    "headers": {
      "Content-Type": "application/json"
    },
    "body": "{\"message\": \"Hello, World!\"}"
  }
}
```

**Response**:
```json
{
  "content": [
    {
      "type": "text",
      "text": "Status: 200 OK\nHeaders: {\"content-type\": \"application/json\", ...}\nBody: {\"message\": \"Hello, World!\"}"
    }
  ]
}
```

**Features**:
- Supports all HTTP methods
- Automatic redirect following
- Timeout protection (30 seconds)
- SSL/TLS support
- Custom headers
- Request/response body handling

## Tool Schema

Each tool provides a JSON Schema for its input parameters. You can retrieve this information using the `tools/list` MCP method:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/list",
  "params": {}
}
```

## Security Considerations

### File System Access
- Tools can read/write any files the server process can access
- Consider running the server as a restricted user
- Use containerization for isolation
- Be cautious with `shell_execute` - it can run arbitrary commands

### Network Access
- `http_request` can access any URL
- Consider firewall rules and network restrictions
- Be aware of SSRF (Server-Side Request Forgery) vulnerabilities

### Resource Limits
- `shell_execute` has a 30-second timeout by default
- File operations are subject to OS limits
- HTTP requests have a 30-second timeout

## Error Handling

All tools return structured error responses:

```json
{
  "content": [
    {
      "type": "text",
      "text": "Error: File not found: /nonexistent/file.txt"
    }
  ],
  "isError": true
}
```

## Testing Tools

### Using the Test Client

```bash
# Build test client
cargo run --example test_client

# Or test manually with curl-like tools
```

### MCP Protocol Testing

```bash
# List available tools
{"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}

# Call a tool
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "echo",
    "arguments": {"message": "test"}
  }
}
```

## Customization

The built-in tools are designed to be production-ready but can be supplemented with custom tools for specific use cases. See the [Server Guide](server.md) for information on adding custom tools.

## Performance Notes

- File operations are asynchronous and non-blocking
- HTTP requests use connection pooling
- Shell commands run in isolated processes
- All operations have appropriate timeouts to prevent hanging

## Troubleshooting

### Tool Not Found
- Ensure the server started successfully
- Check server logs for registration errors
- Verify tool name spelling

### Permission Errors
- Check file permissions for file operations
- Verify network access for HTTP requests
- Ensure shell commands are executable

### Timeout Errors
- Increase timeout values for long-running operations
- Check system resources for performance issues
- Verify network connectivity for HTTP requests