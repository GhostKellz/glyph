use crate::protocol::{
    GlyphError, Result, JsonRpcMessage, JsonRpcRequest, JsonRpcResponse, JsonRpcNotification,
    McpError, RequestId, Implementation, ServerCapabilities, InitializeRequest, InitializeResult,
    ProtocolVersion,
};
use crate::transport::{Transport, TransportServer};
use crate::server::{
    ServerBuilder, RequestHandler, SessionManager, ToolRegistry, ResourceRegistry, PromptRegistry,
};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

#[derive(Debug)]
pub struct Server {
    capabilities: ServerCapabilities,
    server_info: Implementation,
    session_manager: Arc<RwLock<SessionManager>>,
    tool_registry: Arc<RwLock<ToolRegistry>>,
    resource_registry: Arc<RwLock<ResourceRegistry>>,
    prompt_registry: Arc<RwLock<PromptRegistry>>,
    handler: Arc<RequestHandler>,
    initialized: Arc<std::sync::atomic::AtomicBool>,
}

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    pub(crate) fn new(
        capabilities: ServerCapabilities,
        server_info: Implementation,
        session_manager: SessionManager,
        tool_registry: ToolRegistry,
        resource_registry: ResourceRegistry,
        prompt_registry: PromptRegistry,
    ) -> Self {
        let session_manager = Arc::new(RwLock::new(session_manager));
        let tool_registry = Arc::new(RwLock::new(tool_registry));
        let resource_registry = Arc::new(RwLock::new(resource_registry));
        let prompt_registry = Arc::new(RwLock::new(prompt_registry));

        let handler = Arc::new(RequestHandler::new(
            session_manager.clone(),
            tool_registry.clone(),
            resource_registry.clone(),
            prompt_registry.clone(),
        ));

        Self {
            capabilities,
            server_info,
            session_manager,
            tool_registry,
            resource_registry,
            prompt_registry,
            handler,
            initialized: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub async fn run_with_transport<T: Transport + 'static>(&self, mut transport: T) -> Result<()> {
        info!("Starting MCP server with transport");

        loop {
            match transport.receive().await? {
                Some(message) => {
                    if let Err(e) = self.handle_message(&mut transport, message).await {
                        error!("Error handling message: {}", e);
                    }
                }
                None => {
                    info!("Transport closed, shutting down server");
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn run_with_server<S>(&self, mut server: S) -> Result<()>
    where
        S: TransportServer + 'static,
        S::Connection: Transport + 'static,
    {
        info!("Starting MCP server, waiting for connections");

        loop {
            match server.accept().await {
                Ok(connection) => {
                    let handler = self.clone_for_connection();
                    tokio::spawn(async move {
                        if let Err(e) = handler.handle_connection(connection).await {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    fn clone_for_connection(&self) -> Self {
        Self {
            capabilities: self.capabilities.clone(),
            server_info: self.server_info.clone(),
            session_manager: self.session_manager.clone(),
            tool_registry: self.tool_registry.clone(),
            resource_registry: self.resource_registry.clone(),
            prompt_registry: self.prompt_registry.clone(),
            handler: self.handler.clone(),
            initialized: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    async fn handle_connection<T: Transport>(&self, mut transport: T) -> Result<()> {
        info!("New client connection established");

        while !transport.is_closed() {
            match transport.receive().await? {
                Some(message) => {
                    if let Err(e) = self.handle_message(&mut transport, message).await {
                        error!("Error handling message: {}", e);
                        // Don't break on error, continue processing
                    }
                }
                None => {
                    debug!("Client disconnected");
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_message<T: Transport>(
        &self,
        transport: &mut T,
        message: JsonRpcMessage,
    ) -> Result<()> {
        debug!("Received message: {:?}", message);

        match message {
            JsonRpcMessage::Request(request) => {
                self.handle_request(transport, request).await
            }
            JsonRpcMessage::Notification(notification) => {
                self.handle_notification(notification).await
            }
            JsonRpcMessage::Response(response) => {
                warn!("Received unexpected response message: {:?}", response);
                Ok(())
            }
        }
    }

    async fn handle_request<T: Transport>(
        &self,
        transport: &mut T,
        request: JsonRpcRequest<serde_json::Value>,
    ) -> Result<()> {
        let id = request.id.clone();
        let method = request.method.clone();

        // Handle initialize specially
        if method == "initialize" {
            return self.handle_initialize_request(transport, request).await;
        }

        // Check if server is initialized for other requests
        if !self.initialized.load(std::sync::atomic::Ordering::SeqCst) {
            let error = McpError::invalid_request("Server not initialized");
            let response = JsonRpcResponse::error(id, error);
            transport.send(JsonRpcMessage::Response(response)).await?;
            return Ok(());
        }

        // Handle the request
        match self.handler.handle_request(request).await {
            Ok(result) => {
                let response = JsonRpcResponse::success(id, result);
                transport.send(JsonRpcMessage::Response(response)).await?;
            }
            Err(error) => {
                let response = JsonRpcResponse::error(id, error);
                transport.send(JsonRpcMessage::Response(response)).await?;
            }
        }

        Ok(())
    }

    async fn handle_initialize_request<T: Transport>(
        &self,
        transport: &mut T,
        request: JsonRpcRequest<serde_json::Value>,
    ) -> Result<()> {
        let id = request.id.clone();

        let init_request: InitializeRequest = match serde_json::from_value(
            request.params.unwrap_or(serde_json::Value::Null)
        ) {
            Ok(req) => req,
            Err(e) => {
                let error = McpError::invalid_params(format!("Invalid initialize request: {}", e));
                let response = JsonRpcResponse::error(id, error);
                transport.send(JsonRpcMessage::Response(response)).await?;
                return Ok(());
            }
        };

        // Validate protocol version
        let negotiated_version = ProtocolVersion::negotiate(
            &init_request.protocol_version,
            &ProtocolVersion::LATEST,
        );

        let negotiated_version = match negotiated_version {
            Some(version) => version,
            None => {
                let error = McpError::new(
                    crate::protocol::StandardErrorCode::ProtocolVersionMismatch,
                    format!(
                        "Unsupported protocol version: {}",
                        init_request.protocol_version
                    ),
                );
                let response = JsonRpcResponse::error(id, error);
                transport.send(JsonRpcMessage::Response(response)).await?;
                return Ok(());
            }
        };

        // Create session
        {
            let mut session_manager = self.session_manager.write().await;
            session_manager.create_session(
                id.clone(),
                init_request.client_info.clone(),
                init_request.capabilities.clone(),
            ).await?;
        }

        // Mark as initialized
        self.initialized.store(true, std::sync::atomic::Ordering::SeqCst);

        // Send initialize result
        let result = InitializeResult {
            protocol_version: negotiated_version,
            capabilities: self.capabilities.clone(),
            server_info: self.server_info.clone(),
            instructions: None,
        };

        let response = JsonRpcResponse::success(id, result);
        transport.send(JsonRpcMessage::Response(response)).await?;

        info!("Client initialized successfully");
        Ok(())
    }

    async fn handle_notification(
        &self,
        notification: JsonRpcNotification<serde_json::Value>,
    ) -> Result<()> {
        debug!("Received notification: {}", notification.method);

        match notification.method.as_str() {
            "notifications/initialized" => {
                info!("Client confirmed initialization");
            }
            "notifications/cancelled" => {
                // Handle request cancellation
                warn!("Request cancellation not yet implemented");
            }
            _ => {
                warn!("Unknown notification method: {}", notification.method);
            }
        }

        Ok(())
    }

    // Public API methods
    pub async fn register_tool<T>(&self, tool: T) -> Result<()>
    where
        T: crate::server::Tool + 'static,
    {
        let mut registry = self.tool_registry.write().await;
        registry.register(Box::new(tool)).await
    }

    pub async fn register_resource_provider<P>(&self, provider: P) -> Result<()>
    where
        P: crate::server::ResourceProvider + 'static,
    {
        let mut registry = self.resource_registry.write().await;
        registry.register(Box::new(provider)).await
    }

    pub async fn register_prompt<P>(&self, prompt: P) -> Result<()>
    where
        P: crate::server::PromptProvider + 'static,
    {
        let mut registry = self.prompt_registry.write().await;
        registry.register(Box::new(prompt)).await
    }

    pub async fn list_tools(&self) -> Result<Vec<crate::protocol::Tool>> {
        let registry = self.tool_registry.read().await;
        registry.list_tools().await
    }

    pub async fn list_resources(&self) -> Result<Vec<crate::protocol::Resource>> {
        let registry = self.resource_registry.read().await;
        registry.list_resources().await
    }

    pub async fn list_prompts(&self) -> Result<Vec<crate::protocol::Prompt>> {
        let registry = self.prompt_registry.read().await;
        registry.list_prompts().await
    }

    pub fn capabilities(&self) -> &ServerCapabilities {
        &self.capabilities
    }

    pub fn server_info(&self) -> &Implementation {
        &self.server_info
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized.load(std::sync::atomic::Ordering::SeqCst)
    }
}