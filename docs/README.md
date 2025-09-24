# Glyph Documentation

**Enterprise-grade MCP server binary and library for AI assistants**

Welcome to the Glyph documentation! This directory contains comprehensive guides, examples, and API documentation for the Glyph MCP server and library.

## Quick Navigation

### Getting Started
- [Installation Guide](installation.md) - How to add Glyph to your project
- [Quick Start](quickstart.md) - Get up and running in 5 minutes
- [Architecture Overview](architecture.md) - Understanding Glyph's design

### Guides
- [Server Guide](guides/server.md) - Building MCP servers with Glyph
- [Binary Usage](guides/binary.md) - Using the Glyph MCP server binary
- [Built-in Tools](guides/tools.md) - Available built-in tools
- [Transport Guide](guides/transports.md) - Working with different transport layers

### Examples
- [Basic Examples](examples/basic.md) - Simple server and client examples
- [Integration Examples](examples/integrations.md) - MCP client integration

### API Reference
- [Protocol Types](api/protocol.md) - Core MCP protocol types
- [Server API](api/server.md) - Server framework API
- [Client API](api/client.md) - Client library API
- [Transport API](api/transport.md) - Transport layer API

### Advanced Topics
- [Security](advanced/security.md) - Authentication, authorization, audit logging
- [Observability](advanced/observability.md) - Tracing, metrics, logging
- [Performance](advanced/performance.md) - Optimization and benchmarking
- [FFI Integration](advanced/ffi.md) - Integrating with other languages

## Project Structure

```
glyph/
├── src/
│   ├── protocol/          # MCP protocol implementation
│   ├── transport/         # Transport layer (stdio, WebSocket, HTTP)
│   ├── server/           # Server framework
│   ├── client/           # Client library
│   └── main.rs           # CLI binary
├── docs/                 # Documentation (you are here)
├── examples/             # Example applications
├── target/release/glyph  # Release binary
└── tests/               # Integration tests
```

## Key Features

- **Ready-to-Use Binary**: Pre-built MCP server with 7 built-in tools
- **Full MCP Protocol Support**: Complete implementation of the Model Context Protocol
- **Multiple Transports**: stdio, WebSocket, HTTP/1.1, HTTP/2 (planned)
- **Built-in Tools**: File operations, shell execution, HTTP requests, and more
- **Type Safety**: Strong typing throughout with comprehensive error handling
- **Enterprise Ready**: Authentication, audit logging, observability
- **High Performance**: Async/await with Tokio for maximum throughput
- **FFI Ready**: Clean C ABI for integration with other languages

## What is MCP?

The Model Context Protocol (MCP) is a standardized way for AI applications to securely access external tools and data sources. It provides:

- **Tool Calling**: Execute functions with typed parameters
- **Resource Access**: Read files, databases, APIs with proper permissions
- **Prompt Templates**: Reusable prompt patterns with variable substitution
- **Streaming**: Real-time data and progress updates
- **Security**: Authentication, authorization, and audit trails

## Why Glyph?

Glyph is designed for production use with enterprise requirements:

- **Reliability**: Comprehensive error handling and graceful degradation
- **Security**: Built-in authentication, consent mechanisms, audit logging
- **Scalability**: Support for thousands of concurrent connections
- **Observability**: OpenTelemetry integration, structured logging, metrics
- **Interoperability**: FFI bindings for Zig, Python, and other languages
- **Ready to Use**: Pre-built binary with production-ready tools

## Quick Start with Binary

The fastest way to get started is with the Glyph binary:

```bash
# Download and run the MCP server
cargo install glyph
glyph serve

# Or build from source
git clone https://github.com/ghostkellz/glyph
cd glyph
cargo build --release
./target/release/glyph serve
```

The server starts on `ws://127.0.0.1:7331` with 7 built-in tools ready to use.

## Community

- **GitHub**: [https://github.com/ghostkellz/glyph](https://github.com/ghostkellz/glyph)
- **Issues**: [Report bugs and request features](https://github.com/ghostkellz/glyph/issues)
- **Discussions**: [Community Q&A and ideas](https://github.com/ghostkellz/glyph/discussions)

## License

Glyph is licensed under the MIT License. See [LICENSE](../LICENSE) for details.

---

**Next Steps:**
- New to MCP? Start with the [Quick Start guide](quickstart.md)
- Want to use the binary? Check the [Binary Usage guide](guides/binary.md)
- Building a custom server? See the [Server Guide](guides/server.md)
- Need to integrate with existing code? See [FFI Integration](advanced/ffi.md)