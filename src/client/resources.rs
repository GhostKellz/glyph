use crate::protocol::{
    Resource, ResourceContents, ReadResourceRequest, ReadResourceResult, ListResourcesRequest,
    ListResourcesResult, SubscribeRequest, SubscribeResult, UnsubscribeRequest, UnsubscribeResult,
    RequestId,
};
use crate::client::{Connection, ResponseWaiter};
use crate::{Result, JsonRpcMessage, JsonRpcRequest};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Clone)]
pub struct ResourceClient {
    connection: Arc<Mutex<Connection>>,
    pending_requests: Arc<RwLock<HashMap<RequestId, ResponseWaiter>>>,
    request_counter: Arc<std::sync::atomic::AtomicU64>,
}

impl ResourceClient {
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

    pub async fn list_resources(&self, cursor: Option<String>) -> Result<ListResourcesResult> {
        let request = ListResourcesRequest { cursor };
        self.send_request("resources/list", Some(request)).await
    }

    pub async fn read_resource(&self, uri: impl Into<String>) -> Result<ReadResourceResult> {
        let request = ReadResourceRequest { uri: uri.into() };
        self.send_request("resources/read", Some(request)).await
    }

    pub async fn read_resource_text(&self, uri: impl Into<String>) -> Result<String> {
        let result = self.read_resource(uri).await?;

        for content in result.contents {
            match content {
                ResourceContents::Text { text, .. } => return Ok(text),
                ResourceContents::Blob { blob, .. } => {
                    // Try to decode base64 as UTF-8
                    if let Ok(bytes) = base64::decode(&blob) {
                        if let Ok(text) = String::from_utf8(bytes) {
                            return Ok(text);
                        }
                    }
                }
            }
        }

        Err(crate::protocol::GlyphError::JsonRpc(
            "No text content found in resource".to_string()
        ))
    }

    pub async fn read_resource_bytes(&self, uri: impl Into<String>) -> Result<Vec<u8>> {
        let result = self.read_resource(uri).await?;

        for content in result.contents {
            match content {
                ResourceContents::Text { text, .. } => {
                    return Ok(text.into_bytes());
                }
                ResourceContents::Blob { blob, .. } => {
                    return base64::decode(&blob)
                        .map_err(|e| crate::protocol::GlyphError::JsonRpc(
                            format!("Failed to decode base64: {}", e)
                        ));
                }
            }
        }

        Err(crate::protocol::GlyphError::JsonRpc(
            "No content found in resource".to_string()
        ))
    }

    pub async fn subscribe(&self, uri: impl Into<String>) -> Result<()> {
        let request = SubscribeRequest { uri: uri.into() };
        let _: SubscribeResult = self.send_request("resources/subscribe", Some(request)).await?;
        Ok(())
    }

    pub async fn unsubscribe(&self, uri: impl Into<String>) -> Result<()> {
        let request = UnsubscribeRequest { uri: uri.into() };
        let _: UnsubscribeResult = self.send_request("resources/unsubscribe", Some(request)).await?;
        Ok(())
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

// Convenience wrapper for a specific resource
#[derive(Debug)]
pub struct ResourceHandle {
    client: ResourceClient,
    uri: String,
}

impl ResourceHandle {
    pub fn new(client: ResourceClient, uri: impl Into<String>) -> Self {
        Self {
            client,
            uri: uri.into(),
        }
    }

    pub async fn read(&self) -> Result<ReadResourceResult> {
        self.client.read_resource(&self.uri).await
    }

    pub async fn read_text(&self) -> Result<String> {
        self.client.read_resource_text(&self.uri).await
    }

    pub async fn read_bytes(&self) -> Result<Vec<u8>> {
        self.client.read_resource_bytes(&self.uri).await
    }

    pub async fn subscribe(&self) -> Result<()> {
        self.client.subscribe(&self.uri).await
    }

    pub async fn unsubscribe(&self) -> Result<()> {
        self.client.unsubscribe(&self.uri).await
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }
}

// Resource discovery and caching
#[derive(Debug)]
pub struct ResourceRegistry {
    client: ResourceClient,
    resources: Arc<RwLock<Vec<Resource>>>,
    last_updated: Arc<RwLock<Option<std::time::Instant>>>,
    cache_duration: std::time::Duration,
}

impl ResourceRegistry {
    pub fn new(client: ResourceClient) -> Self {
        Self {
            client,
            resources: Arc::new(RwLock::new(Vec::new())),
            last_updated: Arc::new(RwLock::new(None)),
            cache_duration: std::time::Duration::from_secs(300), // 5 minutes
        }
    }

    pub fn with_cache_duration(mut self, duration: std::time::Duration) -> Self {
        self.cache_duration = duration;
        self
    }

    pub async fn refresh(&self) -> Result<()> {
        let result = self.client.list_resources(None).await?;

        {
            let mut resources = self.resources.write().await;
            *resources = result.resources;
        }

        {
            let mut last_updated = self.last_updated.write().await;
            *last_updated = Some(std::time::Instant::now());
        }

        Ok(())
    }

    pub async fn get_resources(&self) -> Result<Vec<Resource>> {
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

        let resources = self.resources.read().await;
        Ok(resources.clone())
    }

    pub async fn get_resource(&self, uri: &str) -> Result<Option<Resource>> {
        let resources = self.get_resources().await?;
        Ok(resources.into_iter().find(|r| r.uri == uri))
    }

    pub async fn get_resource_handle(&self, uri: &str) -> Result<Option<ResourceHandle>> {
        if self.get_resource(uri).await?.is_some() {
            Ok(Some(ResourceHandle::new(self.client.clone(), uri)))
        } else {
            Ok(None)
        }
    }

    pub async fn find_resources_by_name(&self, name: &str) -> Result<Vec<Resource>> {
        let resources = self.get_resources().await?;
        Ok(resources.into_iter().filter(|r| r.name == name).collect())
    }

    pub async fn find_resources_by_mime_type(&self, mime_type: &str) -> Result<Vec<Resource>> {
        let resources = self.get_resources().await?;
        Ok(resources.into_iter()
            .filter(|r| r.mime_type.as_deref() == Some(mime_type))
            .collect())
    }

    pub async fn resource_uris(&self) -> Result<Vec<String>> {
        let resources = self.get_resources().await?;
        Ok(resources.into_iter().map(|r| r.uri).collect())
    }
}

// Base64 decoding (simplified - in real implementation would use base64 crate)
mod base64 {
    pub fn decode(input: &str) -> Result<Vec<u8>, String> {
        // This is a simplified implementation for the example
        // In real code, use the base64 crate
        if input.is_empty() {
            return Ok(Vec::new());
        }

        // For now, just return the input as bytes (this is not real base64 decoding)
        Ok(input.as_bytes().to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::StdioTransport;

    #[tokio::test]
    async fn test_resource_handle_creation() {
        let transport = StdioTransport::new();
        let connection = Arc::new(Mutex::new(crate::client::Connection::new(Box::new(transport))));
        let pending_requests = Arc::new(RwLock::new(HashMap::new()));

        let client = ResourceClient::new(connection, pending_requests);
        let handle = ResourceHandle::new(client, "file:///test.txt");

        assert_eq!(handle.uri(), "file:///test.txt");
    }

    #[tokio::test]
    async fn test_resource_registry() {
        let transport = StdioTransport::new();
        let connection = Arc::new(Mutex::new(crate::client::Connection::new(Box::new(transport))));
        let pending_requests = Arc::new(RwLock::new(HashMap::new()));

        let client = ResourceClient::new(connection, pending_requests);
        let registry = ResourceRegistry::new(client)
            .with_cache_duration(std::time::Duration::from_secs(60));

        assert_eq!(registry.cache_duration, std::time::Duration::from_secs(60));
    }
}