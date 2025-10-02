# Glyph ↔ GhostLLM Integration

Provider passthrough tools for OpenAI, Anthropic, and Gemini through GhostLLM proxy.

## Features

- **Multi-Provider Support**: OpenAI, Anthropic Claude, Google Gemini
- **Cost Tracking**: Automatic USD cost calculation per request
- **Rate Limiting**: Aligned with GhostLLM proxy policies
- **Auth Management**: Centralized API key handling through GhostLLM
- **Usage Monitoring**: Token usage and cost metadata in every response

## Usage

### 1. Register All Providers

```rust
use glyph::server::Server;
use glyph_ghostllm::register_all_providers;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = Server::builder()
        .for_websocket("127.0.0.1:7331")
        .await?;

    // Register OpenAI, Anthropic, and Gemini tools
    register_all_providers(
        server.server(),
        "https://ghostllm.example.com",
        "your-api-key",
    ).await?;

    server.run().await
}
```

### 2. Call OpenAI

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "openai_chat",
    "arguments": {
      "model": "gpt-4-turbo",
      "messages": [
        {
          "role": "system",
          "content": "You are a helpful assistant."
        },
        {
          "role": "user",
          "content": "Explain quantum computing in one sentence."
        }
      ],
      "temperature": 0.7,
      "max_tokens": 100
    }
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Quantum computing harnesses quantum mechanical phenomena..."
      }
    ],
    "_meta": {
      "provider": "openai",
      "model": "gpt-4-turbo",
      "usage": {
        "prompt_tokens": 23,
        "completion_tokens": 15,
        "total_tokens": 38
      },
      "cost_usd": 0.00068,
      "finish_reason": "stop"
    }
  }
}
```

### 3. Call Anthropic Claude

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "anthropic_chat",
    "arguments": {
      "model": "claude-3-sonnet",
      "messages": [
        {
          "role": "user",
          "content": "Write a haiku about code."
        }
      ],
      "max_tokens": 1024
    }
  }
}
```

### 4. Call Google Gemini

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "gemini_chat",
    "arguments": {
      "model": "gemini-pro",
      "messages": [
        {
          "role": "user",
          "content": "Summarize the benefits of Rust."
        }
      ]
    }
  }
}
```

## Cost Tracking

All tools automatically calculate and return cost in USD based on current provider pricing:

| Provider | Model | Prompt (per 1M tokens) | Completion (per 1M tokens) |
|----------|-------|------------------------|----------------------------|
| OpenAI | gpt-4 | $30.00 | $60.00 |
| OpenAI | gpt-4-turbo | $10.00 | $30.00 |
| OpenAI | gpt-3.5-turbo | $0.50 | $1.50 |
| Anthropic | claude-3-opus | $15.00 | $75.00 |
| Anthropic | claude-3-sonnet | $3.00 | $15.00 |
| Anthropic | claude-3-haiku | $0.25 | $1.25 |
| Gemini | gemini-pro | $0.50 | $1.50 |
| Gemini | gemini-ultra | $10.00 | $30.00 |

## GhostLLM Proxy Setup

### Start GhostLLM Proxy

```bash
# Using GhostLLM server
ghostllm serve \
  --openai-key $OPENAI_API_KEY \
  --anthropic-key $ANTHROPIC_API_KEY \
  --gemini-key $GEMINI_API_KEY \
  --port 8080
```

### Configure Glyph

```rust
use glyph_ghostllm::*;

let openai = OpenAITool::new(
    "http://localhost:8080",
    "ghostllm-api-key"
);

server.register_tool(openai).await?;
```

## Rate Limiting

GhostLLM handles rate limiting centrally. Glyph tools respect these limits:

```rust
// Rate limit metadata included in errors
{
  "error": {
    "code": -32000,
    "message": "Rate limit exceeded",
    "data": {
      "retry_after": 60,
      "limit": "10 requests/minute"
    }
  }
}
```

## Authentication

### API Key Flow

```
Client ──► Glyph MCP ──► GhostLLM Proxy ──► Provider API
          (tool call)    (with API key)     (authenticated)
```

### Environment Variables

```bash
export GHOSTLLM_URL="https://ghostllm.example.com"
export GHOSTLLM_API_KEY="your-key-here"

glyph serve --transport stdio
```

## Integration Example

Complete server with all providers:

```rust
use glyph::server::Server;
use glyph_ghostllm::*;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let ghostllm_url = env::var("GHOSTLLM_URL")?;
    let api_key = env::var("GHOSTLLM_API_KEY")?;

    let server = Server::builder()
        .with_server_info("glyph-ghostllm-server", "1.0.0")
        .for_websocket("127.0.0.1:7331")
        .await?;

    // Register all provider tools
    register_all_providers(server.server(), ghostllm_url, api_key).await?;

    tracing::info!("Server ready with OpenAI, Anthropic, and Gemini tools");
    server.run().await
}
```

## Monitoring

Track costs across all providers:

```bash
# Query tool usage
curl -X POST http://localhost:7331 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "openai_chat",
      "arguments": {...}
    }
  }'

# Extract cost from response._meta.cost_usd
```

## See Also

- [Integration Contract](../../docs/INTEGRATION_CONTRACT.md)
- [GhostLLM Documentation](../../archive/ghostllm/README.md)
- [Cost Optimization Guide](./COST_OPTIMIZATION.md)
