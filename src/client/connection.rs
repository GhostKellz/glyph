use crate::protocol::JsonRpcMessage;
use crate::Result;
use crate::transport::Transport;
use std::fmt::Debug;

#[derive(Debug)]
pub struct Connection {
    transport: Box<dyn Transport>,
}

impl Connection {
    pub fn new(transport: Box<dyn Transport>) -> Self {
        Self { transport }
    }

    pub async fn send(&mut self, message: JsonRpcMessage) -> Result<()> {
        self.transport.send(message).await
    }

    pub async fn receive(&mut self) -> Result<Option<JsonRpcMessage>> {
        self.transport.receive().await
    }

    pub async fn close(&mut self) -> Result<()> {
        self.transport.close().await
    }

    pub fn is_closed(&self) -> bool {
        self.transport.is_closed()
    }
}