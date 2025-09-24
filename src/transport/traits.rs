use async_trait::async_trait;
use crate::protocol::JsonRpcMessage;
use crate::Result;
use std::fmt::Debug;

#[async_trait]
pub trait Transport: Send + Sync + Debug {
    async fn send(&mut self, message: JsonRpcMessage) -> Result<()>;
    async fn receive(&mut self) -> Result<Option<JsonRpcMessage>>;
    async fn close(&mut self) -> Result<()>;
    fn is_closed(&self) -> bool;
}

#[async_trait]
pub trait TransportServer: Send + Sync + Debug {
    type Connection: Transport;

    async fn accept(&mut self) -> Result<Self::Connection>;
    async fn close(&mut self) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub read_timeout: Option<std::time::Duration>,
    pub write_timeout: Option<std::time::Duration>,
    pub max_message_size: Option<usize>,
    pub ping_interval: Option<std::time::Duration>,
    pub ping_timeout: Option<std::time::Duration>,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            read_timeout: Some(std::time::Duration::from_secs(30)),
            write_timeout: Some(std::time::Duration::from_secs(10)),
            max_message_size: Some(16 * 1024 * 1024), // 16MB
            ping_interval: Some(std::time::Duration::from_secs(30)),
            ping_timeout: Some(std::time::Duration::from_secs(10)),
        }
    }
}

impl TransportConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_read_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.read_timeout = Some(timeout);
        self
    }

    pub fn with_write_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.write_timeout = Some(timeout);
        self
    }

    pub fn with_max_message_size(mut self, size: usize) -> Self {
        self.max_message_size = Some(size);
        self
    }

    pub fn with_ping_interval(mut self, interval: std::time::Duration) -> Self {
        self.ping_interval = Some(interval);
        self
    }

    pub fn with_ping_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.ping_timeout = Some(timeout);
        self
    }

    pub fn no_timeouts(mut self) -> Self {
        self.read_timeout = None;
        self.write_timeout = None;
        self.ping_timeout = None;
        self
    }
}

#[derive(Debug, Clone)]
pub enum TransportType {
    Stdio,
    WebSocket { url: String },
    Http { url: String },
}