use crate::protocol::{
    GlyphError, JsonRpcMessage, JsonRpcRequest, JsonRpcResponse, JsonRpcNotification,
    RequestId, Implementation, ClientCapabilities, ServerCapabilities, InitializeRequest,
    InitializeResult, ProtocolVersion,
};
use crate::{Error, Result};
use crate::client::{ClientBuilder, Connection, ToolClient, ResourceClient, PromptClient};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{RwLock, Mutex, oneshot};
use tracing::{debug, error, info, warn};

pub type ResponseWaiter = oneshot::Sender<Result<serde_json::Value>>;

#[derive(Debug)]
pub struct Client {
    connection: Arc<Mutex<Connection>>,
    request_counter: AtomicU64,
    pending_requests: Arc<RwLock<HashMap<RequestId, ResponseWaiter>>>,
    server_capabilities: Arc<RwLock<Option<ServerCapabilities>>>,
    server_info: Arc<RwLock<Option<Implementation>>>,
    client_info: Implementation,
    capabilities: ClientCapabilities,
    initialized: Arc<std::sync::atomic::AtomicBool>,
    tools: ToolClient,
    resources: ResourceClient,
    prompts: PromptClient,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    pub async fn connect_stdio() -> Result<Self> {
        Self::builder().connect_stdio().await
    }

    pub async fn connect_websocket(url: &str) -> Result<Self> {
        Self::builder().connect_websocket(url).await
    }

    pub async fn connect_http(url: &str) -> Result<Self> {
        Self::builder().connect_http(url).await
    }

    pub(crate) fn new(
        connection: Connection,
        client_info: Implementation,
        capabilities: ClientCapabilities,
    ) -> Self {
        let connection = Arc::new(Mutex::new(connection));
        let pending_requests = Arc::new(RwLock::new(HashMap::new()));

        let tools = ToolClient::new(connection.clone(), pending_requests.clone());
        let resources = ResourceClient::new(connection.clone(), pending_requests.clone());
        let prompts = PromptClient::new(connection.clone(), pending_requests.clone());

        Self {
            connection,
            request_counter: AtomicU64::new(1),
            pending_requests,
            server_capabilities: Arc::new(RwLock::new(None)),
            server_info: Arc::new(RwLock::new(None)),
            client_info,
            capabilities,
            initialized: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            tools,
            resources,
            prompts,
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        let request = InitializeRequest {
            protocol_version: ProtocolVersion::LATEST,
            capabilities: self.capabilities.clone(),
            client_info: self.client_info.clone(),
        };

        let response: InitializeResult = self.send_request("initialize", Some(request)).await?;

        // Store server capabilities and info
        {
            let mut caps = self.server_capabilities.write().await;
            *caps = Some(response.capabilities);
        }
        {
            let mut info = self.server_info.write().await;
            *info = Some(response.server_info);
        }

        self.initialized.store(true, Ordering::SeqCst);

        // Send initialized notification
        self.send_notification("notifications/initialized", None::<()>).await?;

        info!("Client initialized successfully with protocol version {}", response.protocol_version);
        Ok(())
    }

    pub async fn send_request<T, R>(&self, method: &str, params: Option<T>) -> Result<R>
    where
        T: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let id = RequestId::Number(self.request_counter.fetch_add(1, Ordering::SeqCst) as i64);

        let params = match params {
            Some(p) => Some(serde_json::to_value(p)?),
            None => None,
        };

        let request = JsonRpcRequest::new(id.clone(), method, params);

        // Create response waiter
        let (tx, rx) = oneshot::channel();
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
            .map_err(|_| GlyphError::JsonRpc("Request cancelled".to_string()))??;

        // Deserialize response
        Ok(serde_json::from_value::<R>(response_value)?)
    }

    pub async fn send_notification<T>(&self, method: &str, params: Option<T>) -> Result<()>
    where
        T: serde::Serialize,
    {
        let params = match params {
            Some(p) => Some(serde_json::to_value(p)?),
            None => None,
        };

        let notification = JsonRpcNotification::new(method, params);

        let mut conn = self.connection.lock().await;
        conn.send(JsonRpcMessage::Notification(notification)).await
    }

    pub async fn run_message_loop(&self) -> Result<()> {
        loop {
            let message = {
                let mut conn = self.connection.lock().await;
                conn.receive().await?
            };

            match message {
                Some(msg) => {
                    if let Err(e) = self.handle_message(msg).await {
                        error!("Error handling message: {}", e);
                    }
                }
                None => {
                    info!("Connection closed");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_message(&self, message: JsonRpcMessage) -> Result<()> {
        match message {
            JsonRpcMessage::Response(response) => {
                self.handle_response(response).await
            }
            JsonRpcMessage::Notification(notification) => {
                self.handle_notification(notification).await
            }
            JsonRpcMessage::Request(request) => {
                warn!("Received unexpected request from server: {}", request.method);
                Ok(())
            }
        }
    }

    async fn handle_response(&self, response: JsonRpcResponse<serde_json::Value>) -> Result<()> {
        let waiter = {
            let mut pending = self.pending_requests.write().await;
            pending.remove(&response.id)
        };

        if let Some(waiter) = waiter {
            let result: Result<serde_json::Value> = if let Some(result) = response.result {
                Ok(result)
            } else if let Some(error) = response.error {
                Err(GlyphError::Mcp(error).into())
            } else {
                Err(GlyphError::JsonRpc("Response missing result and error".to_string()).into())
            };

            let _ = waiter.send(result); // Ignore if receiver dropped
        } else {
            warn!("Received response for unknown request ID: {:?}", response.id);
        }

        Ok(())
    }

    async fn handle_notification(&self, notification: JsonRpcNotification<serde_json::Value>) -> Result<()> {
        debug!("Received notification: {}", notification.method);

        match notification.method.as_str() {
            "notifications/progress" => {
                // Handle progress notifications
                debug!("Progress notification received");
            }
            "notifications/message" => {
                // Handle logging messages
                debug!("Server message notification received");
            }
            "notifications/tools/list_changed" => {
                info!("Server tools list changed");
            }
            "notifications/resources/list_changed" => {
                info!("Server resources list changed");
            }
            "notifications/prompts/list_changed" => {
                info!("Server prompts list changed");
            }
            _ => {
                debug!("Unknown notification method: {}", notification.method);
            }
        }

        Ok(())
    }

    pub async fn ping(&self) -> Result<()> {
        let _: crate::protocol::PingResult = self.send_request("ping", None::<()>).await?;
        Ok(())
    }

    pub async fn close(&self) -> Result<()> {
        let mut conn = self.connection.lock().await;
        conn.close().await
    }

    // Accessor methods
    pub fn tools(&self) -> &ToolClient {
        &self.tools
    }

    pub fn resources(&self) -> &ResourceClient {
        &self.resources
    }

    pub fn prompts(&self) -> &PromptClient {
        &self.prompts
    }

    pub async fn server_capabilities(&self) -> Option<ServerCapabilities> {
        self.server_capabilities.read().await.clone()
    }

    pub async fn server_info(&self) -> Option<Implementation> {
        self.server_info.read().await.clone()
    }

    pub fn client_info(&self) -> &Implementation {
        &self.client_info
    }

    pub fn capabilities(&self) -> &ClientCapabilities {
        &self.capabilities
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized.load(Ordering::SeqCst)
    }

    pub async fn is_connected(&self) -> bool {
        let conn = self.connection.lock().await;
        !conn.is_closed()
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        // Cancel all pending requests
        if let Ok(mut pending) = self.pending_requests.try_write() {
            for (_, waiter) in pending.drain() {
                let _ = waiter.send(Err(Error::ConnectionClosed));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::StdioTransport;

    #[tokio::test]
    async fn test_client_creation() {
        let transport = StdioTransport::new();
        let connection = Connection::new(Box::new(transport));
        let client_info = Implementation::new("test-client", "1.0.0");
        let capabilities = ClientCapabilities::new();

        let client = Client::new(connection, client_info, capabilities);

        assert!(!client.is_initialized());
        assert_eq!(client.client_info().name, "test-client");
    }
}