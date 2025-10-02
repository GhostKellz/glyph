# Installation Guide

This guide covers all methods for installing and deploying Glyph MCP.

## Table of Contents
- [From Source](#from-source)
- [Binary Releases](#binary-releases)
- [As a Rust Crate](#as-a-rust-crate)
- [Docker](#docker)
- [System Requirements](#system-requirements)

---

## From Source

### Prerequisites
- Rust 1.75 or later
- Git

### Steps

1. **Clone the repository:**
   ```bash
   git clone https://github.com/ghostkellz/glyph
   cd glyph
   ```

2. **Build the binary:**
   ```bash
   cargo build --release
   ```

3. **Install to system (optional):**
   ```bash
   cargo install --path .
   ```

4. **Verify installation:**
   ```bash
   ./target/release/glyph --version
   # or if installed:
   glyph --version
   ```

---

## Binary Releases

Pre-compiled binaries are available for Linux, macOS, and Windows.

### Download

Visit the [releases page](https://github.com/ghostkellz/glyph/releases) and download the binary for your platform:

- **Linux**: `glyph-linux-x64.tar.gz`
- **macOS**: `glyph-macos-x64.tar.gz` (Intel) or `glyph-macos-arm64.tar.gz` (Apple Silicon)
- **Windows**: `glyph-windows-x64.zip`

### Install

**Linux/macOS:**
```bash
tar xzf glyph-*.tar.gz
sudo mv glyph /usr/local/bin/
glyph --version
```

**Windows:**
1. Extract the zip file
2. Add the directory to your PATH
3. Run `glyph.exe --version` in Command Prompt

---

## As a Rust Crate

Use Glyph as a library in your Rust project.

### Add to Cargo.toml

```toml
[dependencies]
glyph = { git = "https://github.com/ghostkellz/glyph", tag = "v0.1.0" }
# or when published to crates.io:
# glyph = "0.1"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
async-trait = "0.1"
```

### Basic Server Example

```rust
use glyph::server::{Server, Tool, ToolCtx, ToolResult};
use glyph::protocol::Content;
use serde_json::json;

#[derive(Clone)]
struct HelloTool;

#[glyph::async_trait]
impl Tool for HelloTool {
    fn name(&self) -> &str { "hello" }

    async fn call(&self, _ctx: &ToolCtx, _input: json::Value) -> ToolResult<json::Value> {
        Ok(json!({"message": "Hello from Glyph!"}))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut server = Server::builder()
        .transport_stdio()
        .build()
        .await?;

    server.register(HelloTool);
    server.run().await
}
```

### Basic Client Example

```rust
use glyph::client::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::connect_ws("ws://localhost:7331").await?;

    let result = client.tool("hello")
        .invoke(json!({}))
        .await?;

    println!("{}", result);
    Ok(())
}
```

See the [quickstart guide](quickstart.md) and [API documentation](https://docs.rs/glyph) for more examples.

---

## Docker

Run Glyph in a container for easy deployment.

### Quick Start

```bash
docker run -p 7331:7331 ghcr.io/ghostkellz/glyph:latest
```

### Using Docker Compose

Create a `docker-compose.yml`:

```yaml
version: '3.8'

services:
  glyph:
    image: ghcr.io/ghostkellz/glyph:latest
    ports:
      - "7331:7331"
    environment:
      - RUST_LOG=info
    restart: unless-stopped
```

Run with:
```bash
docker-compose up -d
```

### Build Your Own Image

```dockerfile
FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/glyph /usr/local/bin/glyph

EXPOSE 7331

CMD ["glyph", "serve", "--address", "0.0.0.0:7331"]
```

Build and run:
```bash
docker build -t glyph .
docker run -p 7331:7331 glyph
```

---

## System Requirements

### Minimum Requirements
- **RAM**: 128 MB
- **Disk**: 50 MB for binary
- **OS**: Linux (kernel 3.2+), macOS 10.12+, or Windows 10+

### Recommended for Production
- **RAM**: 512 MB+
- **CPU**: 2+ cores
- **Network**: Low latency connection for WebSocket clients

### Rust Toolchain (for building from source)
- **MSRV** (Minimum Supported Rust Version): 1.75.0
- Recommended: Latest stable Rust

---

## Next Steps

- [Quickstart Guide](quickstart.md) - Get started with Glyph
- [Server Binary Guide](guides/server_binary.md) - Learn CLI usage
- [Transport Guide](guides/transports.md) - Configure transports
- [Built-in Tools](guides/builtin_tools.md) - Explore available tools
- [Architecture Overview](architecture.md) - Understand the design

## Troubleshooting

### Build Errors

**"linker not found"**: Install build tools
```bash
# Ubuntu/Debian
sudo apt-get install build-essential

# macOS
xcode-select --install
```

**"cargo: command not found"**: [Install Rust](https://rustup.rs/)

### Runtime Errors

**"Address already in use"**: Change the port
```bash
glyph serve --address 127.0.0.1:8080
```

**Permission denied on Linux**: Run as user or adjust permissions
```bash
# Don't use privileged ports (<1024) or use sudo
glyph serve --address 127.0.0.1:7331
```

For more help, see [GitHub Issues](https://github.com/ghostkellz/glyph/issues).
