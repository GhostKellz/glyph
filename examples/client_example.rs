use glyph::client::Client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::init();

    // This would connect to a WebSocket server in a real scenario
    // let client = Client::connect_ws("ws://localhost:7331").await?;

    // For now, demonstrate the API structure
    println!("Glyph client example - WebSocket connection would go here");
    println!("Usage:");
    println!("  let client = Client::connect_ws(\"ws://localhost:7331\").await?;");
    println!("  let result = client.tool(\"read_file\")");
    println!("    .invoke(serde_json::json!({{ \"path\": \"/etc/hosts\" }}))");
    println!("    .await?;");
    println!("  println!(\"{{}}\", result);");

    Ok(())
}