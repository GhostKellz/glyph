use crate::{
    protocol::{self, *},
    transport::{Transport, ChannelTransport},
    Error, Result,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

pub type ToolResult<T> = Result<T>;

#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> Option<&'static str> {
        None
    }
    fn input_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": true
        })
    }
    async fn call(&self, ctx: &ToolCtx, input: Value) -> ToolResult<Value>;
}

pub struct ToolCtx {
    pub guard: PolicyGuard,
    pub request_id: Option<String>,
}

pub struct PolicyGuard {
    // Future: implement actual policy checking
}

impl PolicyGuard {
    pub fn new() -> Self {
        Self {}
    }

    pub fn require(&self, _permission: &str) -> crate::Result<()> {
        // Future: implement actual permission checking
        Ok(())
    }
}

pub struct Server {
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
    transport: Option<Box<dyn Transport>>,
    capabilities: ServerCapabilities,
}

impl Server {
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            transport: None,
            capabilities: ServerCapabilities {
                tools: Some(ListCapability { list_changed: Some(true) }),
                ..Default::default()
            },
        }
    }

    pub async fn register<T: Tool + 'static>(&mut self, tool: T) {
        let name = tool.name().to_string();
        let mut tools = self.tools.write().await;
        tools.insert(name.clone(), Arc::new(tool));
        info!("Registered tool: {}", name);
    }

    #[instrument(skip(self))]
    pub async fn run(&mut self) -> Result<()> {
        let mut transport = self.transport.take()
            .ok_or_else(|| Error::internal("No transport configured"))?;

        info!("Starting MCP server");

        loop {
            match transport.receive().await {
                Ok(Some(value)) => {
                    if let Ok(request) = serde_json::from_value::<JsonRpcRequest>(value.clone()) {
                        let response = self.handle_request(request).await;
                        if let Some(resp) = response {
                            transport.send(serde_json::to_value(resp)?).await?;
                        }
                    } else if let Ok(_notification) = serde_json::from_value::<NotificationMessage>(value) {
                        debug!("Received notification");
                    } else {
                        warn!("Invalid JSON-RPC message");
                    }
                }
                Ok(None) => {
                    info!("Client disconnected");
                    break;
                }
                Err(e) => {
                    error!("Transport error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    async fn handle_request(&self, request: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = request.id.clone();

        match request.method.as_str() {
            "initialize" => {
                let result = self.handle_initialize(request.params).await;
                Some(self.create_response(id, result))
            }
            "tools/list" => {
                let result = self.handle_tools_list().await;
                Some(self.create_response(id, result))
            }
            "tools/call" => {
                let result = self.handle_tool_call(request.params).await;
                Some(self.create_response(id, result))
            }
            _ => {
                let error = JsonRpcError::new(
                    JsonRpcError::METHOD_NOT_FOUND,
                    format!("Method not found: {}", request.method),
                );
                Some(JsonRpcResponse::error(id, error))
            }
        }
    }

    async fn handle_initialize(&self, params: Option<Value>) -> Result<Value> {
        let _request: InitializeRequest = if let Some(p) = params {
            serde_json::from_value(p)?
        } else {
            return Err(Error::InvalidRequest("Missing initialize parameters".to_string()));
        };

        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: self.capabilities.clone(),
            server_info: Implementation {
                name: "glyph".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };

        Ok(serde_json::to_value(result)?)
    }

    async fn handle_tools_list(&self) -> Result<Value> {
        let tools = self.tools.read().await;
        let tool_list: Vec<protocol::Tool> = tools
            .values()
            .map(|tool| protocol::Tool {
                name: tool.name().to_string(),
                description: tool.description().map(|s| s.to_string()),
                input_schema: tool.input_schema(),
            })
            .collect();

        let result = ToolsListResult { tools: tool_list };
        Ok(serde_json::to_value(result)?)
    }

    async fn handle_tool_call(&self, params: Option<Value>) -> Result<Value> {
        let request: ToolCallRequest = if let Some(p) = params {
            serde_json::from_value(p)?
        } else {
            return Err(Error::InvalidRequest("Missing tool call parameters".to_string()));
        };

        let tools = self.tools.read().await;
        let tool = tools.get(&request.name)
            .ok_or_else(|| Error::ToolNotFound { name: request.name.clone() })?;

        let ctx = ToolCtx {
            guard: PolicyGuard::new(),
            request_id: None,
        };

        let arguments = request.arguments.unwrap_or(Value::Object(serde_json::Map::new()));

        match tool.call(&ctx, arguments).await {
            Ok(result) => {
                let tool_result = ToolCallResult {
                    content: vec![crate::protocol::ToolResult::Text {
                        text: serde_json::to_string_pretty(&result)?,
                    }],
                    is_error: None,
                };
                Ok(serde_json::to_value(tool_result)?)
            }
            Err(e) => {
                let tool_result = ToolCallResult {
                    content: vec![crate::protocol::ToolResult::Text {
                        text: e.to_string(),
                    }],
                    is_error: Some(true),
                };
                Ok(serde_json::to_value(tool_result)?)
            }
        }
    }

    fn create_response(&self, id: Option<RequestId>, result: Result<Value>) -> JsonRpcResponse {
        match result {
            Ok(value) => JsonRpcResponse::success(id, value),
            Err(e) => {
                let error = JsonRpcError::new(JsonRpcError::INTERNAL_ERROR, e.to_string());
                JsonRpcResponse::error(id, error)
            }
        }
    }
}

pub struct ServerBuilder {
    server: Server,
    transport_type: Option<TransportType>,
}

enum TransportType {
    Stdio,
    #[cfg(feature = "websocket")]
    WebSocket { port: u16 },
}

impl ServerBuilder {
    fn new() -> Self {
        Self {
            server: Server::new(),
            transport_type: None,
        }
    }

    pub fn transport_stdio(mut self) -> Self {
        self.transport_type = Some(TransportType::Stdio);
        self
    }

    #[cfg(feature = "websocket")]
    pub fn transport_websocket(mut self, port: u16) -> Self {
        self.transport_type = Some(TransportType::WebSocket { port });
        self
    }

    pub async fn build(mut self) -> Result<Server> {
        match self.transport_type {
            Some(TransportType::Stdio) => {
                // For now, use a channel transport for testing
                let (transport, _) = ChannelTransport::new();
                self.server.transport = Some(Box::new(transport));
            }
            #[cfg(feature = "websocket")]
            Some(TransportType::WebSocket { port: _ }) => {
                // Future: implement WebSocket server transport
                return Err(Error::internal("WebSocket server not yet implemented"));
            }
            None => {
                return Err(Error::internal("No transport configured"));
            }
        }

        Ok(self.server)
    }
}