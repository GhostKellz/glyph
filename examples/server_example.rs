use glyph::{server::{Server, Tool, ToolCtx, ToolResult}, json};
use async_trait::async_trait;

#[derive(serde::Deserialize)]
struct ReadFileInput {
    path: String
}

struct ReadFile;

#[async_trait::async_trait]
impl Tool for ReadFile {
    fn name(&self) -> &'static str {
        "read_file"
    }

    fn description(&self) -> Option<&'static str> {
        Some("Read contents of a file")
    }

    fn input_schema(&self) -> json::Value {
        json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                }
            },
            "required": ["path"]
        })
    }

    async fn call(&self, ctx: &ToolCtx, input: json::Value) -> ToolResult<json::Value> {
        ctx.guard.require("fs.read")?; // optional consent policy
        let args: ReadFileInput = serde_json::from_value(input)?;

        match tokio::fs::read_to_string(&args.path).await {
            Ok(data) => Ok(json::json!({ "contents": data })),
            Err(e) => Ok(json::json!({ "error": e.to_string() }))
        }
    }
}

struct EchoTool;

#[async_trait::async_trait]
impl Tool for EchoTool {
    fn name(&self) -> &'static str {
        "echo"
    }

    fn description(&self) -> Option<&'static str> {
        Some("Echo back the input")
    }

    async fn call(&self, _ctx: &ToolCtx, input: json::Value) -> ToolResult<json::Value> {
        Ok(json::json!({ "echo": input }))
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::init();

    let mut srv = Server::builder().transport_stdio().build().await?;
    srv.register(ReadFile).await;
    srv.register(EchoTool).await;

    println!("Starting Glyph MCP server with stdio transport");
    println!("Registered tools: read_file, echo");

    // In a real scenario, this would run the server
    // srv.run().await

    println!("Server example completed - would run indefinitely in real usage");
    Ok(())
}