use glyph::{server::{Server, Tool, ToolCtx, ToolResult}, json};
use async_trait::async_trait;

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
    tracing_subscriber::fmt::init();

    println!("Glyph v{}", env!("CARGO_PKG_VERSION"));
    println!("Enterprise-grade Rust library for Model Context Protocol (MCP)");

    let mut srv = Server::builder().transport_stdio().build().await?;
    srv.register(EchoTool).await;

    println!("Starting MCP server with stdio transport...");
    Ok(srv.run().await?)
}
