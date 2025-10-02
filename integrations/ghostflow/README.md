# Glyph ↔ GhostFlow Integration

Bidirectional integration between Glyph MCP and GhostFlow workflow engine.

## Features

- **MCP Tools as Nodes**: Use any Glyph MCP tool as a GhostFlow workflow node
- **Workflows as Prompts**: Expose GhostFlow workflows as MCP prompt templates
- **Flow Execution**: Execute entire workflows through MCP tool calls
- **Auto-discovery**: Automatically export all MCP tools to GhostFlow node library

## Usage

### 1. Export Glyph Tools to GhostFlow

```rust
use glyph_ghostflow::export_tools_as_nodes;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Export all Glyph tools as GhostFlow nodes
    let nodes = export_tools_as_nodes("ws://localhost:7331").await?;

    // Save to GhostFlow node library
    for node in nodes {
        println!("Node: {} ({})", node.name, node.type_);
    }

    Ok(())
}
```

### 2. Use MCP Tool in GhostFlow Workflow

```rust
use glyph_ghostflow::{McpToolNode, NodePosition};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create MCP tool node
    let calculator = McpToolNode::new("ws://localhost:7331", "calculator").await?;

    // Execute with GhostFlow inputs
    let mut inputs = HashMap::new();
    inputs.insert("operation".to_string(), json!("add"));
    inputs.insert("a".to_string(), json!(5));
    inputs.insert("b".to_string(), json!(3));

    let outputs = calculator.execute(inputs).await?;
    println!("Result: {:?}", outputs);

    Ok(())
}
```

### 3. Import GhostFlow Workflow as MCP Prompt

```rust
use glyph_ghostflow::{Workflow, import_workflow_as_prompt};
use glyph::server::Server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = Server::builder()
        .for_websocket("127.0.0.1:7331")
        .await?;

    // Define workflow
    let workflow = Workflow {
        id: uuid::Uuid::new_v4().to_string(),
        name: "code_review_flow".to_string(),
        description: Some("Multi-step code review workflow".to_string()),
        nodes: vec![
            // ... workflow nodes
        ],
        connections: vec![],
    };

    // Import as MCP prompt
    import_workflow_as_prompt(&server.server(), workflow).await?;

    server.run().await
}
```

### 4. Execute Workflow via MCP

```rust
use glyph_ghostflow::{GhostFlowExecutionTool, Workflow};
use glyph::server::Server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = Server::builder()
        .for_websocket("127.0.0.1:7331")
        .await?;

    let workflow = Workflow {
        // ... define workflow
    };

    // Expose workflow as MCP tool
    let flow_tool = GhostFlowExecutionTool::new(workflow, "ws://localhost:7331").await?;
    server.server().register_tool(flow_tool).await?;

    server.run().await
}
```

## Architecture

```
┌─────────────────┐         ┌─────────────────┐
│  Glyph MCP      │◄───────►│   GhostFlow     │
│  (Tools)        │         │   (Workflows)   │
└─────────────────┘         └─────────────────┘
        │                           │
        │    1. Export Tools        │
        │    as Nodes              │
        ├──────────────────────────►│
        │                           │
        │    2. Execute Tool        │
        │    from Workflow         │
        │◄──────────────────────────│
        │                           │
        │    3. Import Workflow     │
        │    as Prompt             │
        ├──────────────────────────►│
```

## Example: Data Processing Pipeline

```rust
// Create workflow with MCP tool nodes
let workflow = Workflow {
    id: Uuid::new_v4().to_string(),
    name: "data_pipeline".to_string(),
    description: Some("Process data through multiple MCP tools".to_string()),
    nodes: vec![
        FlowNode {
            id: "read".to_string(),
            type_: "mcp_tool".to_string(),
            name: "read_file".to_string(),
            parameters: hashmap! {
                "path".to_string() => json!("data.json")
            },
            position: NodePosition { x: 0.0, y: 0.0 },
        },
        FlowNode {
            id: "process".to_string(),
            type_: "mcp_tool".to_string(),
            name: "http_request".to_string(),
            parameters: hashmap! {
                "url".to_string() => json!("https://api.example.com/process")
            },
            position: NodePosition { x: 200.0, y: 0.0 },
        },
        FlowNode {
            id: "write".to_string(),
            type_: "mcp_tool".to_string(),
            name: "write_file".to_string(),
            parameters: hashmap! {
                "path".to_string() => json!("output.json")
            },
            position: NodePosition { x: 400.0, y: 0.0 },
        },
    ],
    connections: vec![
        FlowConnection {
            source: "read".to_string(),
            source_handle: "output".to_string(),
            target: "process".to_string(),
            target_handle: "input".to_string(),
        },
        FlowConnection {
            source: "process".to_string(),
            source_handle: "output".to_string(),
            target: "write".to_string(),
            target_handle: "input".to_string(),
        },
    ],
};

// Execute workflow
let executor = FlowExecutor::new("ws://localhost:7331").await?;
let result = executor.execute_workflow(&workflow, HashMap::new()).await?;
```

## Integration with GhostFlow UI

In your GhostFlow frontend:

```typescript
// Fetch available MCP tools
const response = await fetch('http://localhost:3000/api/mcp/nodes');
const mcpNodes = await response.json();

// Add to node palette
mcpNodes.forEach(node => {
  palette.addNode({
    type: 'mcp_tool',
    name: node.name,
    icon: 'mcp-logo',
    inputs: node.inputSchema.properties,
    outputs: { result: 'any' },
  });
});
```

## See Also

- [Integration Contract](../../docs/INTEGRATION_CONTRACT.md)
- [Glyph Documentation](../../docs/README.md)
- [GhostFlow Documentation](../../archive/ghostflow/README.md)
