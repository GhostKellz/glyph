/// GhostFlow Integration for Glyph MCP
///
/// This integration allows Glyph MCP tools to be used as GhostFlow workflow nodes,
/// and GhostFlow workflows to be exposed as MCP prompts.

use glyph::client::Client;
use glyph::server::{Server, Tool, PromptProvider, ToolContext};
use glyph::protocol::{CallToolResult, Content, ToolInputSchema, GetPromptResult, PromptMessage};
use glyph::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use async_trait::async_trait;
use uuid::Uuid;

// ============================================================================
// GhostFlow Node Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowNode {
    pub id: String,
    pub type_: String,
    pub name: String,
    pub parameters: HashMap<String, Value>,
    pub position: NodePosition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowConnection {
    pub source: String,
    pub source_handle: String,
    pub target: String,
    pub target_handle: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub nodes: Vec<FlowNode>,
    pub connections: Vec<FlowConnection>,
}

// ============================================================================
// MCP Tool Node Adapter
// ============================================================================

/// Wraps a Glyph MCP tool as a GhostFlow node
pub struct McpToolNode {
    mcp_client: Client,
    tool_name: String,
    node_id: String,
}

impl McpToolNode {
    pub async fn new(mcp_url: &str, tool_name: impl Into<String>) -> anyhow::Result<Self> {
        let mcp_client = Client::connect_ws(mcp_url).await?;
        Ok(Self {
            mcp_client,
            tool_name: tool_name.into(),
            node_id: Uuid::new_v4().to_string(),
        })
    }

    pub async fn execute(&self, inputs: HashMap<String, Value>) -> anyhow::Result<HashMap<String, Value>> {
        // Convert GhostFlow inputs to MCP tool arguments
        let arguments = json!(inputs);

        // Call MCP tool
        let result = self.mcp_client
            .call_tool(&self.tool_name, Some(arguments))
            .await?;

        // Convert MCP result to GhostFlow outputs
        let mut outputs = HashMap::new();

        // Extract text content
        if let Some(content) = result.content.first() {
            outputs.insert("result".to_string(), json!(content.text()));
        }

        // Include metadata if present
        if let Some(meta) = result.meta {
            outputs.insert("metadata".to_string(), meta);
        }

        outputs.insert("is_error".to_string(), json!(result.is_error.unwrap_or(false)));

        Ok(outputs)
    }

    pub fn to_flow_node(&self, position: NodePosition) -> FlowNode {
        FlowNode {
            id: self.node_id.clone(),
            type_: "mcp_tool".to_string(),
            name: self.tool_name.clone(),
            parameters: HashMap::new(),
            position,
        }
    }
}

// ============================================================================
// GhostFlow Workflow as MCP Prompt
// ============================================================================

/// Exposes a GhostFlow workflow as an MCP prompt template
pub struct WorkflowPrompt {
    workflow: Workflow,
}

impl WorkflowPrompt {
    pub fn new(workflow: Workflow) -> Self {
        Self { workflow }
    }

    fn generate_prompt_from_workflow(&self, args: HashMap<String, String>) -> String {
        let mut prompt = format!("Execute workflow: {}\n\n", self.workflow.name);

        if let Some(desc) = &self.workflow.description {
            prompt.push_str(&format!("Description: {}\n\n", desc));
        }

        prompt.push_str("Workflow steps:\n");
        for (i, node) in self.workflow.nodes.iter().enumerate() {
            prompt.push_str(&format!("{}. {} ({})\n", i + 1, node.name, node.type_));
        }

        prompt.push_str("\nInputs:\n");
        for (key, value) in args.iter() {
            prompt.push_str(&format!("- {}: {}\n", key, value));
        }

        prompt
    }
}

#[async_trait]
impl PromptProvider for WorkflowPrompt {
    fn name(&self) -> &str {
        &self.workflow.name
    }

    fn description(&self) -> Option<&str> {
        self.workflow.description.as_deref()
    }

    async fn get_prompt(&self, arguments: HashMap<String, String>) -> Result<GetPromptResult> {
        let prompt_text = self.generate_prompt_from_workflow(arguments);

        Ok(GetPromptResult {
            description: self.workflow.description.clone(),
            messages: vec![
                PromptMessage {
                    role: glyph::protocol::Role::User,
                    content: Content::text(prompt_text),
                },
            ],
        })
    }
}

// ============================================================================
// GhostFlow Execution Engine
// ============================================================================

pub struct FlowExecutor {
    mcp_client: Client,
}

impl FlowExecutor {
    pub async fn new(mcp_url: &str) -> anyhow::Result<Self> {
        let mcp_client = Client::connect_ws(mcp_url).await?;
        Ok(Self { mcp_client })
    }

    pub async fn execute_workflow(
        &self,
        workflow: &Workflow,
        inputs: HashMap<String, Value>,
    ) -> anyhow::Result<HashMap<String, Value>> {
        let mut execution_context: HashMap<String, Value> = inputs;

        // Execute nodes in topological order (simplified - assumes linear flow)
        for node in &workflow.nodes {
            if node.type_ == "mcp_tool" {
                // Extract tool parameters from node
                let tool_name = &node.name;
                let tool_inputs = node.parameters.clone();

                // Merge with execution context
                let merged_inputs: Value = json!(tool_inputs.iter()
                    .map(|(k, v)| (k.clone(), execution_context.get(k).unwrap_or(v).clone()))
                    .collect::<HashMap<String, Value>>());

                // Call MCP tool
                let result = self.mcp_client
                    .call_tool(tool_name, Some(merged_inputs))
                    .await?;

                // Store result in context
                execution_context.insert(
                    format!("{}_result", node.id),
                    json!(result.content.first().map(|c| c.text()).unwrap_or_default()),
                );
            }
        }

        Ok(execution_context)
    }
}

// ============================================================================
// GhostFlow Tool (exposes workflow execution as MCP tool)
// ============================================================================

pub struct GhostFlowExecutionTool {
    workflow: Workflow,
    executor: FlowExecutor,
}

impl GhostFlowExecutionTool {
    pub async fn new(workflow: Workflow, mcp_url: &str) -> anyhow::Result<Self> {
        let executor = FlowExecutor::new(mcp_url).await?;
        Ok(Self { workflow, executor })
    }
}

#[async_trait]
impl Tool for GhostFlowExecutionTool {
    fn name(&self) -> &str {
        &self.workflow.name
    }

    fn description(&self) -> Option<&str> {
        self.workflow.description.as_deref()
    }

    fn input_schema(&self) -> ToolInputSchema {
        // Generate schema from workflow nodes
        ToolInputSchema {
            schema_type: "object".to_string(),
            properties: Some(HashMap::new()),
            required: None,
            additional: HashMap::new(),
        }
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult> {
        let inputs = args
            .and_then(|v| serde_json::from_value::<HashMap<String, Value>>(v).ok())
            .unwrap_or_default();

        let result = self.executor
            .execute_workflow(&self.workflow, inputs)
            .await
            .map_err(|e| glyph::Error::Protocol(e.to_string()))?;

        Ok(CallToolResult {
            content: vec![Content::text(serde_json::to_string_pretty(&result).unwrap())],
            is_error: None,
            meta: Some(json!({
                "workflow_id": self.workflow.id,
                "nodes_executed": self.workflow.nodes.len(),
            })),
        })
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Register all Glyph tools as GhostFlow nodes
pub async fn export_tools_as_nodes(
    mcp_url: &str,
) -> anyhow::Result<Vec<FlowNode>> {
    let client = Client::connect_ws(mcp_url).await?;
    let tools = client.list_tools().await?;

    let nodes = tools
        .tools
        .iter()
        .enumerate()
        .map(|(i, tool)| FlowNode {
            id: Uuid::new_v4().to_string(),
            type_: "mcp_tool".to_string(),
            name: tool.name.clone(),
            parameters: HashMap::new(),
            position: NodePosition {
                x: 100.0,
                y: 100.0 + (i as f64 * 150.0),
            },
        })
        .collect();

    Ok(nodes)
}

/// Import a GhostFlow workflow as MCP prompt
pub async fn import_workflow_as_prompt(
    server: &Server,
    workflow: Workflow,
) -> anyhow::Result<()> {
    let prompt = WorkflowPrompt::new(workflow);
    server.register_prompt_provider(prompt).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workflow_creation() {
        let workflow = Workflow {
            id: Uuid::new_v4().to_string(),
            name: "test_workflow".to_string(),
            description: Some("Test workflow".to_string()),
            nodes: vec![],
            connections: vec![],
        };

        assert_eq!(workflow.name, "test_workflow");
    }

    #[test]
    fn test_flow_node_creation() {
        let node = FlowNode {
            id: "node1".to_string(),
            type_: "mcp_tool".to_string(),
            name: "echo".to_string(),
            parameters: HashMap::new(),
            position: NodePosition { x: 0.0, y: 0.0 },
        };

        assert_eq!(node.type_, "mcp_tool");
    }
}
