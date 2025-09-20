use crate::{Error, Result};
use serde_json::Value;
use std::pin::Pin;
use std::future::Future;
use tokio::sync::mpsc;

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

#[async_trait::async_trait]
pub trait Transport: Send + Sync {
    async fn send(&mut self, message: Value) -> Result<()>;
    async fn receive(&mut self) -> Result<Option<Value>>;
    async fn close(&mut self) -> Result<()>;
}

pub struct ChannelTransport {
    sender: mpsc::UnboundedSender<Value>,
    receiver: mpsc::UnboundedReceiver<Value>,
}

impl ChannelTransport {
    pub fn new() -> (Self, Self) {
        let (tx1, rx1) = mpsc::unbounded_channel();
        let (tx2, rx2) = mpsc::unbounded_channel();

        (
            Self { sender: tx1, receiver: rx2 },
            Self { sender: tx2, receiver: rx1 },
        )
    }
}

#[async_trait::async_trait]
impl Transport for ChannelTransport {
    async fn send(&mut self, message: Value) -> Result<()> {
        self.sender
            .send(message)
            .map_err(|_| Error::transport("Channel closed"))?;
        Ok(())
    }

    async fn receive(&mut self) -> Result<Option<Value>> {
        Ok(self.receiver.recv().await)
    }

    async fn close(&mut self) -> Result<()> {
        Ok(())
    }
}

#[cfg(feature = "websocket")]
pub mod websocket {
    use super::*;
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::{connect_async, tungstenite::Message, WebSocketStream};
    use url::Url;

    pub struct WebSocketTransport<S> {
        stream: WebSocketStream<S>,
    }

    impl WebSocketTransport<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>> {
        pub async fn connect(url: &str) -> Result<Self> {
            let (stream, _) = connect_async(url)
                .await
                .map_err(|e| Error::websocket(format!("Connection failed: {}", e)))?;

            Ok(Self { stream })
        }
    }

    #[async_trait::async_trait]
    impl<S> Transport for WebSocketTransport<S>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + Sync,
    {
        async fn send(&mut self, message: Value) -> Result<()> {
            let text = serde_json::to_string(&message)?;
            self.stream
                .send(Message::Text(text))
                .await
                .map_err(|e| Error::websocket(format!("Send failed: {}", e)))?;
            Ok(())
        }

        async fn receive(&mut self) -> Result<Option<Value>> {
            match self.stream.next().await {
                Some(Ok(Message::Text(text))) => {
                    let value = serde_json::from_str(&text)?;
                    Ok(Some(value))
                }
                Some(Ok(Message::Close(_))) => Ok(None),
                Some(Ok(_)) => Ok(None), // Ignore binary/ping/pong
                Some(Err(e)) => Err(Error::websocket(format!("Receive failed: {}", e))),
                None => Ok(None),
            }
        }

        async fn close(&mut self) -> Result<()> {
            self.stream
                .close(None)
                .await
                .map_err(|e| Error::websocket(format!("Close failed: {}", e)))?;
            Ok(())
        }
    }
}