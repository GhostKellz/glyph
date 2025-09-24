use serde::{Deserialize, Serialize};
use crate::protocol::{RequestId, McpError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonRpcVersion2_0;

impl JsonRpcVersion2_0 {
    pub const VALUE: &'static str = "2.0";
}

impl Default for JsonRpcVersion2_0 {
    fn default() -> Self {
        Self
    }
}

impl Serialize for JsonRpcVersion2_0 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(Self::VALUE)
    }
}

impl<'de> Deserialize<'de> for JsonRpcVersion2_0 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        if s == Self::VALUE {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom(format!(
                "Expected JSON-RPC version '{}', got '{}'",
                Self::VALUE,
                s
            )))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest<T> {
    pub jsonrpc: JsonRpcVersion2_0,
    pub id: RequestId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<T>,
}

impl<T> JsonRpcRequest<T> {
    pub fn new(id: impl Into<RequestId>, method: impl Into<String>, params: Option<T>) -> Self {
        Self {
            jsonrpc: JsonRpcVersion2_0::default(),
            id: id.into(),
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification<T> {
    pub jsonrpc: JsonRpcVersion2_0,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<T>,
}

impl<T> JsonRpcNotification<T> {
    pub fn new(method: impl Into<String>, params: Option<T>) -> Self {
        Self {
            jsonrpc: JsonRpcVersion2_0::default(),
            method: method.into(),
            params,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: JsonRpcVersion2_0,
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
}

impl<T> JsonRpcResponse<T> {
    pub fn success(id: impl Into<RequestId>, result: T) -> Self {
        Self {
            jsonrpc: JsonRpcVersion2_0::default(),
            id: id.into(),
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: impl Into<RequestId>, error: McpError) -> Self {
        Self {
            jsonrpc: JsonRpcVersion2_0::default(),
            id: id.into(),
            result: None,
            error: Some(error),
        }
    }

    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest<serde_json::Value>),
    Notification(JsonRpcNotification<serde_json::Value>),
    Response(JsonRpcResponse<serde_json::Value>),
}

impl JsonRpcMessage {
    pub fn parse_request<T: for<'de> Deserialize<'de>>(
        &self,
    ) -> Result<JsonRpcRequest<T>, serde_json::Error> {
        match self {
            JsonRpcMessage::Request(req) => {
                let params = match &req.params {
                    Some(p) => Some(serde_json::from_value(p.clone())?),
                    None => None,
                };
                Ok(JsonRpcRequest {
                    jsonrpc: req.jsonrpc.clone(),
                    id: req.id.clone(),
                    method: req.method.clone(),
                    params,
                })
            }
            _ => Err(serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::InvalidData, "Not a request"))),
        }
    }

    pub fn parse_notification<T: for<'de> Deserialize<'de>>(
        &self,
    ) -> Result<JsonRpcNotification<T>, serde_json::Error> {
        match self {
            JsonRpcMessage::Notification(notif) => {
                let params = match &notif.params {
                    Some(p) => Some(serde_json::from_value(p.clone())?),
                    None => None,
                };
                Ok(JsonRpcNotification {
                    jsonrpc: notif.jsonrpc.clone(),
                    method: notif.method.clone(),
                    params,
                })
            }
            _ => Err(serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::InvalidData, "Not a notification"))),
        }
    }

    pub fn parse_response<T: for<'de> Deserialize<'de>>(
        &self,
    ) -> Result<JsonRpcResponse<T>, serde_json::Error> {
        match self {
            JsonRpcMessage::Response(resp) => {
                let result = match &resp.result {
                    Some(r) => Some(serde_json::from_value(r.clone())?),
                    None => None,
                };
                Ok(JsonRpcResponse {
                    jsonrpc: resp.jsonrpc.clone(),
                    id: resp.id.clone(),
                    result,
                    error: resp.error.clone(),
                })
            }
            _ => Err(serde_json::Error::io(std::io::Error::new(std::io::ErrorKind::InvalidData, "Not a response"))),
        }
    }
}