use crate::protocol::{
    Tool, CallToolRequest, CallToolResult, ListToolsRequest, ListToolsResult, RequestId,
};
use crate::client::{Connection, ResponseWaiter};
use crate::{Result, JsonRpcMessage, JsonRpcRequest};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ToolClient {
    connection: Arc<Mutex<Connection>>,
    pending_requests: Arc<RwLock<HashMap<RequestId, ResponseWaiter>>>,
    request_counter: Arc<std::sync::atomic::AtomicU64>,
}

impl ToolClient {
    pub fn new(
        connection: Arc<Mutex<Connection>>,
        pending_requests: Arc<RwLock<HashMap<RequestId, ResponseWaiter>>>,
    ) -> Self {
        Self {
            connection,
            pending_requests,
            request_counter: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    pub async fn list_tools(&self, cursor: Option<String>) -> Result<ListToolsResult> {
        let request = ListToolsRequest { cursor };
        self.send_request("tools/list", Some(request)).await
    }

    pub async fn call_tool(
        &self,
        name: impl Into<String>,
        arguments: Option<Value>,
    ) -> Result<CallToolResult> {
        let request = CallToolRequest {
            name: name.into(),
            arguments,
        };
        self.send_request("tools/call", Some(request)).await
    }

    pub async fn call_tool_typed<T, R>(
        &self,
        name: impl Into<String>,
        arguments: Option<T>,
    ) -> Result<R>
    where
        T: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let arguments = match arguments {
            Some(args) => Some(serde_json::to_value(args)?),
            None => None,
        };

        let result = self.call_tool(name, arguments).await?;

        // If the tool result has structured content, try to deserialize it
        if let Some(meta) = result.meta {
            if let Some(structured) = meta.get("structured_content") {
                return Ok(serde_json::from_value(structured.clone())?);
            }
        }

        // Otherwise, try to deserialize from the first text content
        if let Some(content) = result.content.first() {
            if let crate::protocol::Content::Text { text } = content {
                return Ok(serde_json::from_str(text)?);
            }
        }

        Err(crate::protocol::GlyphError::JsonRpc(
            "Tool result cannot be deserialized to requested type".to_string()
        ))
    }

    async fn send_request<T, R>(&self, method: &str, params: Option<T>) -> Result<R>
    where
        T: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let id = RequestId::Number(
            self.request_counter
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst) as i64
        );

        let params = match params {
            Some(p) => Some(serde_json::to_value(p)?),
            None => None,
        };

        let request = JsonRpcRequest::new(id.clone(), method, params);

        // Create response waiter
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(id.clone(), tx);
        }

        // Send request
        {
            let mut conn = self.connection.lock().await;
            conn.send(JsonRpcMessage::Request(request)).await?;
        }

        // Wait for response
        let response_value = rx.await
            .map_err(|_| crate::protocol::GlyphError::JsonRpc("Request cancelled".to_string()))??;

        // Deserialize response
        serde_json::from_value(response_value)
            .map_err(|e| crate::protocol::GlyphError::Serialization(e))
    }
}

// Convenience wrapper for a specific tool
#[derive(Debug)]
pub struct ToolHandle {
    client: ToolClient,
    name: String,
}

impl ToolHandle {
    pub fn new(client: ToolClient, name: impl Into<String>) -> Self {
        Self {
            client,
            name: name.into(),
        }
    }

    pub async fn call(&self, arguments: Option<Value>) -> Result<CallToolResult> {
        self.client.call_tool(&self.name, arguments).await
    }

    pub async fn call_typed<T, R>(&self, arguments: Option<T>) -> Result<R>
    where
        T: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        self.client.call_tool_typed(&self.name, arguments).await
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

// Tool discovery and caching
#[derive(Debug)]
pub struct ToolRegistry {
    client: ToolClient,
    tools: Arc<RwLock<Vec<Tool>>>,
    last_updated: Arc<RwLock<Option<std::time::Instant>>>,
    cache_duration: std::time::Duration,
}

impl ToolRegistry {
    pub fn new(client: ToolClient) -> Self {
        Self {
            client,
            tools: Arc::new(RwLock::new(Vec::new())),
            last_updated: Arc::new(RwLock::new(None)),
            cache_duration: std::time::Duration::from_secs(300), // 5 minutes
        }
    }

    pub fn with_cache_duration(mut self, duration: std::time::Duration) -> Self {
        self.cache_duration = duration;
        self
    }

    pub async fn refresh(&self) -> Result<()> {
        let result = self.client.list_tools(None).await?;

        {
            let mut tools = self.tools.write().await;
            *tools = result.tools;
        }

        {
            let mut last_updated = self.last_updated.write().await;
            *last_updated = Some(std::time::Instant::now());
        }

        Ok(())
    }

    pub async fn get_tools(&self) -> Result<Vec<Tool>> {
        let should_refresh = {
            let last_updated = self.last_updated.read().await;
            match *last_updated {
                Some(time) => time.elapsed() > self.cache_duration,
                None => true,
            }
        };

        if should_refresh {
            self.refresh().await?;
        }

        let tools = self.tools.read().await;
        Ok(tools.clone())
    }

    pub async fn get_tool(&self, name: &str) -> Result<Option<Tool>> {
        let tools = self.get_tools().await?;
        Ok(tools.into_iter().find(|t| t.name == name))
    }

    pub async fn get_tool_handle(&self, name: &str) -> Result<Option<ToolHandle>> {
        if self.get_tool(name).await?.is_some() {
            Ok(Some(ToolHandle::new(self.client.clone(), name)))
        } else {
            Ok(None)
        }
    }

    pub async fn tool_names(&self) -> Result<Vec<String>> {
        let tools = self.get_tools().await?;
        Ok(tools.into_iter().map(|t| t.name).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::StdioTransport;

    #[tokio::test]
    async fn test_tool_handle_creation() {
        let transport = StdioTransport::new();
        let connection = Arc::new(Mutex::new(crate::client::Connection::new(Box::new(transport))));
        let pending_requests = Arc::new(RwLock::new(HashMap::new()));

        let client = ToolClient::new(connection, pending_requests);
        let handle = ToolHandle::new(client, "test_tool");

        assert_eq!(handle.name(), "test_tool");
    }

    #[tokio::test]
    async fn test_tool_registry() {
        let transport = StdioTransport::new();
        let connection = Arc::new(Mutex::new(crate::client::Connection::new(Box::new(transport))));
        let pending_requests = Arc::new(RwLock::new(HashMap::new()));

        let client = ToolClient::new(connection, pending_requests);
        let registry = ToolRegistry::new(client)
            .with_cache_duration(std::time::Duration::from_secs(60));

        // Note: This test doesn't actually call refresh() since we don't have a real server
        assert_eq!(registry.cache_duration, std::time::Duration::from_secs(60));
    }
}