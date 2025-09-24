use glyph::server::{Server, Tool};
use glyph::protocol::{CallToolResult, Content, ToolInputSchema};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

#[derive(serde::Deserialize)]
struct ReadFileInput {
    path: String
}

struct ReadFile;

#[async_trait]
impl Tool for ReadFile {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> Option<&str> {
        Some("Read contents of a file")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut properties = HashMap::new();
        properties.insert("path".to_string(), serde_json::json!({
            "type": "string",
            "description": "Path to the file to read"
        }));

        ToolInputSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["path".to_string()]),
            additional: HashMap::new(),
        }
    }

    async fn call(&self, args: Option<Value>) -> glyph::Result<CallToolResult> {
        let args = args.ok_or_else(|| glyph::Error::Protocol("Missing arguments".to_string()))?;

        let input: ReadFileInput = serde_json::from_value(args)?;

        match tokio::fs::read_to_string(&input.path).await {
            Ok(data) => Ok(CallToolResult {
                content: vec![Content::text(data)],
                is_error: None,
                meta: None,
            }),
            Err(e) => Ok(CallToolResult {
                content: vec![Content::text(format!("Error reading file: {}", e))],
                is_error: Some(true),
                meta: None,
            })
        }
    }
}

struct EchoTool;

#[async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &str {
        "echo"
    }

    fn description(&self) -> Option<&str> {
        Some("Echo back the input")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut properties = HashMap::new();
        properties.insert("message".to_string(), serde_json::json!({
            "type": "string",
            "description": "Message to echo"
        }));

        ToolInputSchema {
            schema_type: "object".to_string(),
            properties: Some(properties),
            required: Some(vec!["message".to_string()]),
            additional: HashMap::new(),
        }
    }

    async fn call(&self, args: Option<Value>) -> glyph::Result<CallToolResult> {
        let args = args.ok_or_else(|| glyph::Error::Protocol("Missing arguments".to_string()))?;

        Ok(CallToolResult {
            content: vec![Content::text(format!("Echo: {}", args))],
            is_error: None,
            meta: None,
        })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let _srv = Server::builder()
        .with_server_info("example-server", "1.0.0")
        .with_tools()
        .for_stdio();

    println!("Starting Glyph MCP server with stdio transport");
    println!("Example completed - would run server in real usage");

    // In a real scenario, this would run the server:
    // srv.run().await?;

    Ok(())
}