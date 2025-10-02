/// Glyph Integration Example
///
/// This example demonstrates how to integrate Glyph MCP into your Rust project.
/// It shows:
/// - Creating a custom MCP server with your own tools
/// - Registering built-in and custom tools
/// - Using multiple transports (stdio and WebSocket)
/// - Setting up proper logging and error handling

use glyph::server::{Server, Tool, ToolContext};
use glyph::protocol::{CallToolResult, Content, ToolInputSchema};
use glyph::Result;
use serde_json::{json, Value};
use std::collections::HashMap;
use async_trait::async_trait;

// ============================================================================
// Custom Tool Example
// ============================================================================

/// A custom tool that performs calculations
struct CalculatorTool;

#[async_trait]
impl Tool for CalculatorTool {
    fn name(&self) -> &str {
        "calculator"
    }

    fn description(&self) -> Option<&str> {
        Some("Perform basic arithmetic calculations")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut props = HashMap::new();
        props.insert("operation".to_string(), json!({
            "type": "string",
            "enum": ["add", "subtract", "multiply", "divide"],
            "description": "The operation to perform"
        }));
        props.insert("a".to_string(), json!({
            "type": "number",
            "description": "First operand"
        }));
        props.insert("b".to_string(), json!({
            "type": "number",
            "description": "Second operand"
        }));

        ToolInputSchema {
            schema_type: "object".to_string(),
            properties: Some(props),
            required: Some(vec!["operation".to_string(), "a".to_string(), "b".to_string()]),
            additional: HashMap::new(),
        }
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult> {
        let args = args.ok_or_else(|| glyph::Error::Protocol("Missing arguments".into()))?;

        let operation = args["operation"].as_str()
            .ok_or_else(|| glyph::Error::Protocol("Missing operation".into()))?;
        let a = args["a"].as_f64()
            .ok_or_else(|| glyph::Error::Protocol("Invalid operand a".into()))?;
        let b = args["b"].as_f64()
            .ok_or_else(|| glyph::Error::Protocol("Invalid operand b".into()))?;

        let result = match operation {
            "add" => a + b,
            "subtract" => a - b,
            "multiply" => a * b,
            "divide" => {
                if b == 0.0 {
                    return Ok(CallToolResult {
                        content: vec![Content::text("Error: Division by zero")],
                        is_error: Some(true),
                        meta: None,
                    });
                }
                a / b
            }
            _ => return Err(glyph::Error::Protocol(format!("Unknown operation: {}", operation))),
        };

        Ok(CallToolResult {
            content: vec![Content::text(format!("{} {} {} = {}", a, operation, b, result))],
            is_error: None,
            meta: Some(json!({
                "operation": operation,
                "result": result,
            })),
        })
    }
}

// ============================================================================
// Main Application
// ============================================================================
//
// Note: Resource and Prompt providers can be added similarly.
// See the Glyph source code for server::ResourceProvider and server::PromptProvider traits.
// ============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter("info,glyph=debug")
        .init();

    tracing::info!("Starting Glyph integration example");

    // Determine transport from environment variable
    let transport = std::env::var("TRANSPORT").unwrap_or_else(|_| "stdio".to_string());

    match transport.as_str() {
        "websocket" => {
            let addr = std::env::var("ADDRESS").unwrap_or_else(|_| "127.0.0.1:7331".to_string());
            tracing::info!("Starting WebSocket server on {}", addr);

            let server_wrapper = Server::builder()
                .with_server_info("glyph-integration-example", "0.1.0")
                .for_websocket(&addr)
                .await?;

            // Register custom tools
            tracing::info!("Registering custom calculator tool");
            let server = server_wrapper.server();
            server.register_tool(CalculatorTool).await?;

            tracing::info!("Server ready at ws://{}", addr);
            server_wrapper.run().await?;
        }
        "stdio" => {
            tracing::info!("Starting stdio server");

            let server_wrapper = Server::builder()
                .with_server_info("glyph-integration-example", "0.1.0")
                .for_stdio();

            // Register custom tools
            tracing::info!("Registering custom calculator tool");
            let server = server_wrapper.server();
            server.register_tool(CalculatorTool).await?;

            tracing::info!("Server ready for stdio communication");
            server_wrapper.run().await?;
        }
        _ => {
            return Err(anyhow::anyhow!("Unknown transport: {}", transport));
        }
    }

    Ok(())
}
