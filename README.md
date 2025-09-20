
# Glyph

<div align="center">
  <img src="assets/glyph-logo.png" alt="Glyph Logo" width="160" height="160">

  **Enterprise-grade Rust library for Model Context Protocol (MCP)**

  ![MCP Server](https://img.shields.io/badge/MCP-Server-blue)
  ![MCP Client](https://img.shields.io/badge/MCP-Client-green)
  ![WebSocket](https://img.shields.io/badge/Transport-WebSocket-orange)
  ![HTTP/2](https://img.shields.io/badge/Transport-HTTP%2F2-orange)
  ![Policy Engine](https://img.shields.io/badge/Security-Policy%20Engine-red)
  ![Observability](https://img.shields.io/badge/Monitoring-Tracing%20%2B%20Metrics-purple)
  ![Schema First](https://img.shields.io/badge/API-Schema%20First-yellow)
  ![FFI Ready](https://img.shields.io/badge/Interop-FFI%20Ready-lightgrey)
</div>

**Glyph** is the Rust backbone for MCP in your stack—**server + client + transports + schemas**—built for high throughput and deep observability. Ideal for **GhostLLM**, **GhostFlow**, **Jarvis**, and service backends.

- 🧠 **Full MCP stack**: types, JSON-RPC, capabilities, sessions
- 🛰️ **Transports**: stdio, WebSocket, HTTP/1.1, HTTP/2 (h3 optional)
- 🔐 **Consent/Audit**: policy gates + signed audit log hooks
- 📜 **OpenAPI/JSON-Schema**: first-class tool and resource schemas
- 📈 **Prod-ready**: tokio, tracing, metrics, robust error model
- 🔗 **Interop**: clean FFI surface for Zig (Rune) and other languages

---

## Add to your project

**Cargo.toml**
```toml
[dependencies]
glyph = { git = "https://github.com/ghostkellz/glyph", tag = "v0.1.0" }
# or path/crates.io when published
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
tracing = "0.1"
```

## Quick Start — Client

```rust
use glyph::client::Client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::connect_ws("wss://localhost:7331").await?;
    let out = client.tool("read_file")
        .invoke(serde_json::json!({ "path": "/etc/hosts" }))
        .await?;
    println!("{}", out);
    Ok(())
}
```

## Quick Start — Server Tool

```rust
use glyph::{server::{Server, Tool, ToolCtx, ToolResult}, json};

#[derive(serde::Deserialize)]
struct ReadFileInput { path: String }

struct ReadFile;

#[glyph::async_trait]
impl Tool for ReadFile {
    fn name(&self) -> &'static str { "read_file" }
    async fn call(&self, ctx: &ToolCtx, input: json::Value) -> ToolResult<json::Value> {
        ctx.guard.require("fs.read")?; // optional consent policy
        let args: ReadFileInput = serde_json::from_value(input)?;
        let data = tokio::fs::read_to_string(args.path).await?;
        Ok(json::json!({ "contents": data }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut srv = Server::builder().transport_stdio().build().await?;
    srv.register(ReadFile);
    srv.run().await
}
```

## Features

- **Transports**: stdio, ws, http, optional h3
- **Security**: policy engine hooks (GhostGuard-style), redaction, audit
- **Observability**: tracing, Prometheus metrics, request IDs
- **Schemas**: derive-macros for tools/resources from OpenAPI/JSON-Schema
- **Interop**: glyph-ffi for C ABI → Zig (Rune) can link directly

## Integration Targets

- **[GhostLLM](https://github.com/ghostkellz/ghostllm)**: expose provider tools via MCP; route requests safely
- **[GhostFlow](https://github.com/ghostkellz/ghostflow)**: call MCP tools as nodes; publish flows as MCP tools
- **[Jarvis](https://github.com/ghostkellz/jarvis)**: local ai agent, arch system agent, blockchain agent
- **[Zeke](https://github.com/ghostkellz/zeke)**: local tool host with consent prompts
- **zeke.nvim**: Claude-Code.nvim like plugin for zeke AI systems
- **Wraith/gDNS**: admin operations exposed as audited tools
