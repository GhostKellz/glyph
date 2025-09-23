use async_trait::async_trait;
use crate::protocol::{JsonRpcMessage, GlyphError, Result};
use crate::transport::{Transport, TransportServer, TransportConfig};
use tokio_tungstenite::{
    accept_async, connect_async, tungstenite::Message, WebSocketStream, MaybeTlsStream,
};
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::StreamExt;
use futures_util::{SinkExt, StreamExt as FuturesStreamExt};
use std::sync::Arc;
use url::Url;

#[derive(Debug)]
pub struct WebSocketTransport {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    config: TransportConfig,
    closed: Arc<std::sync::atomic::AtomicBool>,
}

impl WebSocketTransport {
    pub async fn connect(url: &str) -> Result<Self> {
        Self::connect_with_config(url, TransportConfig::default()).await
    }

    pub async fn connect_with_config(url: &str, config: TransportConfig) -> Result<Self> {
        let url = Url::parse(url)
            .map_err(|e| GlyphError::Transport(format!("Invalid URL: {}", e)))?;

        let (stream, _) = connect_async(url).await
            .map_err(|e| GlyphError::Transport(format!("WebSocket connection failed: {}", e)))?;

        Ok(Self {
            stream,
            config,
            closed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    pub fn from_stream(stream: WebSocketStream<MaybeTlsStream<TcpStream>>) -> Self {
        Self::from_stream_with_config(stream, TransportConfig::default())
    }

    pub fn from_stream_with_config(
        stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
        config: TransportConfig,
    ) -> Self {
        Self {
            stream,
            config,
            closed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    async fn send(&mut self, message: JsonRpcMessage) -> Result<()> {
        if self.is_closed() {
            return Err(GlyphError::ConnectionClosed);
        }

        let json = serde_json::to_string(&message)?;

        if let Some(max_size) = self.config.max_message_size {
            if json.len() > max_size {
                return Err(GlyphError::Transport(format!(
                    "Message too large: {} bytes, max: {} bytes",
                    json.len(),
                    max_size
                )));
            }
        }

        let send_future = self.stream.send(Message::Text(json));

        if let Some(timeout) = self.config.write_timeout {
            tokio::time::timeout(timeout, send_future)
                .await
                .map_err(|_| GlyphError::Timeout)?
                .map_err(|e| GlyphError::Transport(format!("WebSocket send error: {}", e)))?;
        } else {
            send_future.await
                .map_err(|e| GlyphError::Transport(format!("WebSocket send error: {}", e)))?;
        }

        Ok(())
    }

    async fn receive(&mut self) -> Result<Option<JsonRpcMessage>> {
        if self.is_closed() {
            return Ok(None);
        }

        let receive_future = async {
            match self.stream.next().await {
                Some(Ok(msg)) => match msg {
                    Message::Text(text) => {
                        if let Some(max_size) = self.config.max_message_size {
                            if text.len() > max_size {
                                return Err(GlyphError::Transport(format!(
                                    "Message too large: {} bytes, max: {} bytes",
                                    text.len(),
                                    max_size
                                )));
                            }
                        }

                        let message: JsonRpcMessage = serde_json::from_str(&text)?;
                        Ok(Some(message))
                    }
                    Message::Binary(data) => {
                        if let Some(max_size) = self.config.max_message_size {
                            if data.len() > max_size {
                                return Err(GlyphError::Transport(format!(
                                    "Message too large: {} bytes, max: {} bytes",
                                    data.len(),
                                    max_size
                                )));
                            }
                        }

                        let text = String::from_utf8(data)
                            .map_err(|e| GlyphError::Transport(format!("Invalid UTF-8: {}", e)))?;
                        let message: JsonRpcMessage = serde_json::from_str(&text)?;
                        Ok(Some(message))
                    }
                    Message::Close(_) => {
                        self.closed.store(true, std::sync::atomic::Ordering::SeqCst);
                        Ok(None)
                    }
                    Message::Ping(data) => {
                        // Respond to ping with pong
                        self.stream.send(Message::Pong(data)).await
                            .map_err(|e| GlyphError::Transport(format!("Pong send error: {}", e)))?;
                        // Continue listening for the next message
                        self.receive().await
                    }
                    Message::Pong(_) => {
                        // Ignore pong messages and continue listening
                        self.receive().await
                    }
                    Message::Frame(_) => {
                        // Raw frames should not be encountered in high-level API
                        self.receive().await
                    }
                },
                Some(Err(e)) => {
                    self.closed.store(true, std::sync::atomic::Ordering::SeqCst);
                    Err(GlyphError::Transport(format!("WebSocket error: {}", e)))
                }
                None => {
                    self.closed.store(true, std::sync::atomic::Ordering::SeqCst);
                    Ok(None)
                }
            }
        };

        if let Some(timeout) = self.config.read_timeout {
            tokio::time::timeout(timeout, receive_future)
                .await
                .map_err(|_| GlyphError::Timeout)?
        } else {
            receive_future.await
        }
    }

    async fn close(&mut self) -> Result<()> {
        self.closed.store(true, std::sync::atomic::Ordering::SeqCst);

        self.stream.close(None).await
            .map_err(|e| GlyphError::Transport(format!("WebSocket close error: {}", e)))?;

        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.closed.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[derive(Debug)]
pub struct WebSocketServer {
    listener: TcpListener,
    config: TransportConfig,
}

impl WebSocketServer {
    pub async fn bind(addr: &str) -> Result<Self> {
        Self::bind_with_config(addr, TransportConfig::default()).await
    }

    pub async fn bind_with_config(addr: &str, config: TransportConfig) -> Result<Self> {
        let listener = TcpListener::bind(addr).await
            .map_err(|e| GlyphError::Transport(format!("Failed to bind to {}: {}", addr, e)))?;

        Ok(Self { listener, config })
    }

    pub fn local_addr(&self) -> Result<std::net::SocketAddr> {
        self.listener.local_addr()
            .map_err(|e| GlyphError::Transport(format!("Failed to get local address: {}", e)))
    }
}

#[async_trait]
impl TransportServer for WebSocketServer {
    type Connection = WebSocketTransport;

    async fn accept(&mut self) -> Result<Self::Connection> {
        let (stream, addr) = self.listener.accept().await
            .map_err(|e| GlyphError::Transport(format!("Failed to accept connection: {}", e)))?;

        tracing::debug!("New WebSocket connection from {}", addr);

        let ws_stream = accept_async(stream).await
            .map_err(|e| GlyphError::Transport(format!("WebSocket handshake failed: {}", e)))?;

        Ok(WebSocketTransport::from_stream_with_config(ws_stream, self.config.clone()))
    }

    async fn close(&mut self) -> Result<()> {
        // TcpListener doesn't need explicit closing
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{JsonRpcRequest, JsonRpcVersion2_0, RequestId};

    #[tokio::test]
    async fn test_websocket_server_creation() -> Result<()> {
        let server = WebSocketServer::bind("127.0.0.1:0").await?;
        let addr = server.local_addr()?;
        assert!(addr.port() > 0);
        Ok(())
    }

    #[tokio::test]
    async fn test_websocket_transport_with_config() {
        let config = TransportConfig::new()
            .with_max_message_size(1024)
            .with_read_timeout(std::time::Duration::from_secs(5));

        // Note: This test doesn't actually connect, just tests config handling
        assert_eq!(config.max_message_size, Some(1024));
    }
}