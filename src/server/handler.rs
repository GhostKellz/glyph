use crate::protocol::{
    JsonRpcRequest, McpError, McpResult, CallToolRequest, ReadResourceRequest,
    ListToolsRequest, ListResourcesRequest, ListPromptsRequest, GetPromptRequest,
    SubscribeRequest, UnsubscribeRequest,
};
use crate::server::{SessionManager, ToolRegistry, ResourceRegistry, PromptRegistry};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;

#[derive(Debug)]
pub struct RequestHandler {
    session_manager: Arc<RwLock<SessionManager>>,
    tool_registry: Arc<RwLock<ToolRegistry>>,
    resource_registry: Arc<RwLock<ResourceRegistry>>,
    prompt_registry: Arc<RwLock<PromptRegistry>>,
}

impl RequestHandler {
    pub fn new(
        session_manager: Arc<RwLock<SessionManager>>,
        tool_registry: Arc<RwLock<ToolRegistry>>,
        resource_registry: Arc<RwLock<ResourceRegistry>>,
        prompt_registry: Arc<RwLock<PromptRegistry>>,
    ) -> Self {
        Self {
            session_manager,
            tool_registry,
            resource_registry,
            prompt_registry,
        }
    }

    pub async fn handle_request(
        &self,
        request: JsonRpcRequest<Value>,
    ) -> Result<Value, McpError> {
        match request.method.as_str() {
            "tools/list" => self.handle_list_tools(request).await,
            "tools/call" => self.handle_call_tool(request).await,
            "resources/list" => self.handle_list_resources(request).await,
            "resources/read" => self.handle_read_resource(request).await,
            "resources/subscribe" => self.handle_subscribe(request).await,
            "resources/unsubscribe" => self.handle_unsubscribe(request).await,
            "prompts/list" => self.handle_list_prompts(request).await,
            "prompts/get" => self.handle_get_prompt(request).await,
            "ping" => self.handle_ping(request).await,
            _ => Err(McpError::method_not_found(&request.method)),
        }
    }

    async fn handle_list_tools(&self, request: JsonRpcRequest<Value>) -> Result<Value, McpError> {
        let req: ListToolsRequest = self.parse_params(request.params)?;
        let registry = self.tool_registry.read().await;
        let tools = registry.list_tools().await
            .map_err(|e| McpError::internal_error(format!("Failed to list tools: {}", e)))?;

        let result = crate::protocol::ListToolsResult {
            tools,
            next_cursor: None, // TODO: Implement pagination
        };

        serde_json::to_value(result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e)))
    }

    async fn handle_call_tool(&self, request: JsonRpcRequest<Value>) -> Result<Value, McpError> {
        let req: CallToolRequest = self.parse_params(request.params)?;
        let registry = self.tool_registry.read().await;
        let result = registry.call_tool(req).await?;

        serde_json::to_value(result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e)))
    }

    async fn handle_list_resources(&self, request: JsonRpcRequest<Value>) -> Result<Value, McpError> {
        let req: ListResourcesRequest = self.parse_params(request.params)?;
        let registry = self.resource_registry.read().await;
        let resources = registry.list_resources().await
            .map_err(|e| McpError::internal_error(format!("Failed to list resources: {}", e)))?;

        let result = crate::protocol::ListResourcesResult {
            resources,
            next_cursor: None, // TODO: Implement pagination
        };

        serde_json::to_value(result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e)))
    }

    async fn handle_read_resource(&self, request: JsonRpcRequest<Value>) -> Result<Value, McpError> {
        let req: ReadResourceRequest = self.parse_params(request.params)?;
        let registry = self.resource_registry.read().await;
        let contents = registry.read_resource(&req.uri).await
            .map_err(|e| McpError::resource_not_found(format!("Failed to read resource: {}", e)))?;

        let result = crate::protocol::ReadResourceResult { contents };

        serde_json::to_value(result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e)))
    }

    async fn handle_subscribe(&self, request: JsonRpcRequest<Value>) -> Result<Value, McpError> {
        let req: SubscribeRequest = self.parse_params(request.params)?;
        let mut registry = self.resource_registry.write().await;
        let session_id = format!("{:?}", request.id);

        registry.subscribe(&req.uri, &session_id).await
            .map_err(|e| McpError::internal_error(format!("Subscription failed: {}", e)))?;

        let result = crate::protocol::SubscribeResult;
        serde_json::to_value(result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e)))
    }

    async fn handle_unsubscribe(&self, request: JsonRpcRequest<Value>) -> Result<Value, McpError> {
        let req: UnsubscribeRequest = self.parse_params(request.params)?;
        let mut registry = self.resource_registry.write().await;
        let session_id = format!("{:?}", request.id);

        registry.unsubscribe(&req.uri, &session_id).await
            .map_err(|e| McpError::internal_error(format!("Unsubscription failed: {}", e)))?;

        let result = crate::protocol::UnsubscribeResult;
        serde_json::to_value(result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e)))
    }

    async fn handle_list_prompts(&self, request: JsonRpcRequest<Value>) -> Result<Value, McpError> {
        let req: ListPromptsRequest = self.parse_params(request.params)?;
        let registry = self.prompt_registry.read().await;
        let prompts = registry.list_prompts().await
            .map_err(|e| McpError::internal_error(format!("Failed to list prompts: {}", e)))?;

        let result = crate::protocol::ListPromptsResult {
            prompts,
            next_cursor: None, // TODO: Implement pagination
        };

        serde_json::to_value(result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e)))
    }

    async fn handle_get_prompt(&self, request: JsonRpcRequest<Value>) -> Result<Value, McpError> {
        let req: GetPromptRequest = self.parse_params(request.params)?;
        let registry = self.prompt_registry.read().await;
        let result = registry.get_prompt(&req.name, req.arguments.unwrap_or_default()).await
            .map_err(|e| McpError::new(
                crate::protocol::StandardErrorCode::PromptExecutionError,
                format!("Failed to get prompt: {}", e)
            ))?;

        serde_json::to_value(result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e)))
    }

    async fn handle_ping(&self, _request: JsonRpcRequest<Value>) -> Result<Value, McpError> {
        let result = crate::protocol::PingResult;
        serde_json::to_value(result)
            .map_err(|e| McpError::internal_error(format!("Serialization error: {}", e)))
    }

    fn parse_params<T>(&self, params: Option<Value>) -> Result<T, McpError>
    where
        T: serde::de::DeserializeOwned + Default,
    {
        match params {
            Some(value) => serde_json::from_value(value)
                .map_err(|e| McpError::invalid_params(format!("Invalid parameters: {}", e))),
            None => Ok(T::default()),
        }
    }
}