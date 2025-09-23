//! High-performance MCP tools powered by Rune (Zig)
//!
//! This module provides MCP tool implementations that leverage the Rune Zig library
//! for ultra-fast execution of text processing, workspace operations, and diagnostics.

use crate::{
    protocol::{ToolInputSchema, CallToolResult, Content},
    server::tools::Tool,
    rune_ffi::Rune,
    Result,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Wrapper for Rune engine that implements MCP Tool trait
pub struct RuneTool {
    name: &'static str,
    description: Option<&'static str>,
    rune: Arc<Mutex<Rune>>,
}

impl RuneTool {
    /// Create a new Rune-powered tool
    pub fn new(name: &'static str, description: Option<&'static str>) -> Result<Self> {
        let mut rune = Rune::new()
            .map_err(|e| crate::Error::ToolExecution(format!("Failed to initialize Rune: {:?}", e)))?;

        // Register the tool with the Rune engine
        rune.register_tool(name, description)
            .map_err(|e| crate::Error::ToolExecution(format!("Failed to register tool: {:?}", e)))?;

        Ok(RuneTool {
            name,
            description,
            rune: Arc::new(Mutex::new(rune)),
        })
    }
}

#[async_trait]
impl Tool for RuneTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> Option<&str> {
        self.description
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut properties = HashMap::new();
        properties.insert(
            "params".to_string(),
            json!({
                "type": "object",
                "description": "Tool-specific parameters"
            })
        );

        ToolInputSchema::object()
            .with_properties(properties)
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult> {
        let rune = self.rune.lock().await;
        let input = args.unwrap_or(Value::Null);

        // Execute the tool via Rune FFI
        match rune.execute_tool(self.name, &input) {
            Ok(result) => {
                // Convert the result to MCP content
                let content = if let Value::String(text) = result {
                    vec![Content::text(text)]
                } else {
                    vec![Content::text(result.to_string())]
                };
                Ok(CallToolResult::success(content))
            }
            Err(e) => {
                let error_msg = format!("Rune tool execution failed: {:?}", e);
                Ok(CallToolResult::error(vec![Content::text(error_msg)]))
            }
        }
    }
}

/// Text Selection Tool - Zero-copy text manipulation
pub fn create_selection_tool() -> Result<RuneTool> {
    RuneTool::new(
        "text_selection",
        Some("High-performance text selection and manipulation with zero-copy operations")
    )
}

/// Workspace Operations Tool - Fast workspace scanning and symbol indexing
pub fn create_workspace_tool() -> Result<RuneTool> {
    RuneTool::new(
        "workspace_ops",
        Some("Lightning-fast workspace scanning, file search, and symbol indexing")
    )
}

/// Diagnostics Tool - Pattern-based error detection and analysis
pub fn create_diagnostics_tool() -> Result<RuneTool> {
    RuneTool::new(
        "diagnostics",
        Some("Advanced diagnostics engine with pattern-based error detection and performance analysis")
    )
}

/// File Operations Tool - High-performance file system operations
pub fn create_file_ops_tool() -> Result<RuneTool> {
    RuneTool::new(
        "file_ops",
        Some("Optimized file operations with memory-mapped I/O and batch processing")
    )
}

/// Convenience function to create all Rune-powered tools
pub fn create_all_rune_tools() -> Result<Vec<Box<dyn Tool>>> {
    let tools: Vec<Box<dyn Tool>> = vec![
        Box::new(create_selection_tool()?),
        Box::new(create_workspace_tool()?),
        Box::new(create_diagnostics_tool()?),
        Box::new(create_file_ops_tool()?),
    ];

    tracing::info!("Created {} Rune-powered MCP tools", tools.len());
    Ok(tools)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_rune_tool_creation() {
        let tool = create_selection_tool().expect("Failed to create selection tool");
        assert_eq!(tool.name(), "text_selection");
        assert!(tool.description().is_some());
    }

    #[tokio::test]
    async fn test_create_all_tools() {
        let tools = create_all_rune_tools().expect("Failed to create tools");
        assert_eq!(tools.len(), 4);

        let tool_names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(tool_names.contains(&"text_selection"));
        assert!(tool_names.contains(&"workspace_ops"));
        assert!(tool_names.contains(&"diagnostics"));
        assert!(tool_names.contains(&"file_ops"));
    }
}