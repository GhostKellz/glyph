use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use glyph::client::{McpClient, ClientConfig};
use glyph::protocol::{InitializeRequest, InitializeResult, CallToolRequest, CallToolResult};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Glyph MCP Server");
    println!("Connecting to ws://127.0.0.1:7331...");

    // Connect to the WebSocket server
    let (ws_stream, _) = connect_async("ws://127.0.0.1:7331").await?;
    println!("✅ Connected to server");

    let (mut write, mut read) = ws_stream.split();

    // Send initialize request
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": "glyph-test-client",
                "version": "0.1.0"
            }
        }
    });

    println!("📤 Sending initialize request...");
    write.send(Message::Text(init_request.to_string())).await?;

    // Read initialize response
    if let Some(message) = read.next().await {
        let msg = message?;
        if let Message::Text(text) = msg {
            let response: Value = serde_json::from_str(&text)?;
            println!("📥 Initialize response: {}", serde_json::to_string_pretty(&response)?);

            if let Some(error) = response.get("error") {
                println!("❌ Initialize failed: {}", error);
                return Ok(());
            }
        }
    }

    // Send initialized notification
    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });

    write.send(Message::Text(initialized_notification.to_string())).await?;
    println!("📤 Sent initialized notification");

    // Test tools/list
    let tools_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    println!("📤 Requesting tools list...");
    write.send(Message::Text(tools_request.to_string())).await?;

    if let Some(message) = read.next().await {
        let msg = message?;
        if let Message::Text(text) = msg {
            let response: Value = serde_json::from_str(&text)?;
            println!("📥 Tools list response: {}", serde_json::to_string_pretty(&response)?);
        }
    }

    // Test echo tool
    let echo_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "tools/call",
        "params": {
            "name": "echo",
            "arguments": {
                "message": "Hello from test client!"
            }
        }
    });

    println!("📤 Testing echo tool...");
    write.send(Message::Text(echo_request.to_string())).await?;

    if let Some(message) = read.next().await {
        let msg = message?;
        if let Message::Text(text) = msg {
            let response: Value = serde_json::from_str(&text)?;
            println!("📥 Echo tool response: {}", serde_json::to_string_pretty(&response)?);
        }
    }

    println!("✅ Test completed successfully!");
    Ok(())
}