use crate::{
    protocol::*,
    transport::{Transport, websocket::WebSocketTransport},
    Error, Result,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, oneshot};
use tracing::{debug, error, instrument};
use uuid::Uuid;

type PendingRequests = Arc<Mutex<HashMap<String, oneshot::Sender<JsonRpcResponse>>>>;

pub struct Client {
    transport: Arc<Mutex<Box<dyn Transport>>>,
    pending: PendingRequests,
    _message_handler: tokio::task::JoinHandle<()>,
}

impl Client {
    #[cfg(feature = "websocket")]
    pub async fn connect_ws(url: &str) -> Result<Self> {
        let transport = WebSocketTransport::connect(url).await?;
        Self::new(Box::new(transport)).await
    }

    pub async fn new(transport: Box<dyn Transport>) -> Result<Self> {
        let transport = Arc::new(Mutex::new(transport));
        let pending: PendingRequests = Arc::new(Mutex::new(HashMap::new()));

        let (_tx, _rx) = mpsc::unbounded_channel::<JsonRpcResponse>();

        // Message handler task
        let transport_clone = transport.clone();
        let pending_clone = pending.clone();
        let message_handler = tokio::spawn(async move {
            let mut transport = transport_clone.lock().await;

            loop {
                match transport.receive().await {
                    Ok(Some(value)) => {
                        if let Ok(response) = serde_json::from_value::<JsonRpcResponse>(value.clone()) {
                            if let Some(id) = &response.id {
                                if let Value::String(id_str) = id {
                                    let mut pending = pending_clone.lock().await;
                                    if let Some(sender) = pending.remove(id_str) {
                                        let _ = sender.send(response);
                                    }
                                }
                            }
                        } else if let Ok(_notification) = serde_json::from_value::<NotificationMessage>(value) {
                            debug!("Received notification");
                        }
                    }
                    Ok(None) => {
                        debug!("Transport closed");
                        break;
                    }
                    Err(e) => {
                        error!("Transport error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(Self {
            transport,
            pending,
            _message_handler: message_handler,
        })
    }

    #[instrument(skip(self))]
    pub async fn initialize(&self, client_name: &str, client_version: &str) -> Result<InitializeResult> {
        let request = InitializeRequest {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: client_name.to_string(),
                version: client_version.to_string(),
            },
        };

        let response = self.send_request("initialize", Some(serde_json::to_value(request)?)).await?;
        let result = serde_json::from_value(response)?;
        Ok(result)
    }

    #[instrument(skip(self))]
    pub async fn list_tools(&self) -> Result<ToolsListResult> {
        let response = self.send_request("tools/list", None).await?;
        let result = serde_json::from_value(response)?;
        Ok(result)
    }

    #[instrument(skip(self, arguments))]
    pub async fn call_tool(&self, name: &str, arguments: Option<Value>) -> Result<ToolCallResult> {
        let request = ToolCallRequest {
            name: name.to_string(),
            arguments,
        };

        let response = self.send_request("tools/call", Some(serde_json::to_value(request)?)).await?;
        let result = serde_json::from_value(response)?;
        Ok(result)
    }

    pub fn tool(&self, name: &str) -> ToolInvoker {
        ToolInvoker {
            client: self,
            name: name.to_string(),
        }
    }

    async fn send_request(&self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = Uuid::new_v4().to_string();
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(Value::String(id.clone())),
            method: method.to_string(),
            params,
        };

        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending.lock().await;
            pending.insert(id, tx);
        }

        {
            let mut transport = self.transport.lock().await;
            transport.send(serde_json::to_value(request)?).await?;
        }

        let response = rx.await.map_err(|_| Error::internal("Request cancelled"))?;

        if let Some(error) = response.error {
            return Err(Error::json_rpc(format!("{}: {}", error.code, error.message)));
        }

        response.result.ok_or_else(|| Error::json_rpc("No result in response"))
    }
}

pub struct ToolInvoker<'a> {
    client: &'a Client,
    name: String,
}

impl<'a> ToolInvoker<'a> {
    pub async fn invoke(&self, arguments: Value) -> Result<ToolCallResult> {
        self.client.call_tool(&self.name, Some(arguments)).await
    }

    pub async fn invoke_empty(&self) -> Result<ToolCallResult> {
        self.client.call_tool(&self.name, None).await
    }
}