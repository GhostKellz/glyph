use async_trait::async_trait;
use crate::protocol::{JsonRpcMessage};
use crate::{Error, Result};
use crate::transport::{Transport, TransportConfig};
use reqwest::Client;
use serde_json;
use std::sync::Arc;
use tokio::sync::mpsc;
use url::Url;

#[derive(Debug)]
pub struct HttpTransport {
    client: Client,
    url: Url,
    config: TransportConfig,
    receiver: mpsc::UnboundedReceiver<JsonRpcMessage>,
    sender: mpsc::UnboundedSender<JsonRpcMessage>,
    closed: Arc<std::sync::atomic::AtomicBool>,
}

impl HttpTransport {
    pub fn new(url: &str) -> Result<Self> {
        Self::with_config(url, TransportConfig::default())
    }

    pub fn with_config(url: &str, config: TransportConfig) -> Result<Self> {
        let url = Url::parse(url)
            .map_err(|e| Error::Transport(format!("Invalid URL: {}", e)))?;

        let client = Client::builder()
            .timeout(config.read_timeout.unwrap_or(std::time::Duration::from_secs(30)))
            .build()
            .map_err(|e| Error::Transport(format!("Failed to create HTTP client: {}", e)))?;

        let (sender, receiver) = mpsc::unbounded_channel();

        Ok(Self {
            client,
            url,
            config,
            receiver,
            sender,
            closed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    pub async fn start_sse_listener(&mut self) -> Result<()> {
        if self.is_closed() {
            return Err(Error::ConnectionClosed.into());
        }

        let mut sse_url = self.url.clone();
        sse_url.set_path("/sse");

        let client = self.client.clone();
        let sender = self.sender.clone();
        let closed = self.closed.clone();

        tokio::spawn(async move {
            let mut response = match client.get(sse_url).send().await {
                Ok(resp) => resp,
                Err(e) => {
                    tracing::error!("Failed to connect to SSE endpoint: {}", e);
                    closed.store(true, std::sync::atomic::Ordering::SeqCst);
                    return;
                }
            };

            while let Some(chunk) = response.chunk().await.transpose() {
                if closed.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }

                match chunk {
                    Ok(bytes) => {
                        let text = match String::from_utf8(bytes.to_vec()) {
                            Ok(t) => t,
                            Err(e) => {
                                tracing::error!("Invalid UTF-8 in SSE chunk: {}", e);
                                continue;
                            }
                        };

                        // Parse SSE format
                        for line in text.lines() {
                            if line.starts_with("data: ") {
                                let json_data = &line[6..]; // Remove "data: " prefix
                                match serde_json::from_str::<JsonRpcMessage>(json_data) {
                                    Ok(message) => {
                                        if sender.send(message).is_err() {
                                            // Receiver was dropped
                                            closed.store(true, std::sync::atomic::Ordering::SeqCst);
                                            return;
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!("Failed to parse SSE JSON: {}", e);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("SSE stream error: {}", e);
                        closed.store(true, std::sync::atomic::Ordering::SeqCst);
                        break;
                    }
                }
            }

            closed.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        Ok(())
    }
}

#[async_trait]
impl Transport for HttpTransport {
    async fn send(&mut self, message: JsonRpcMessage) -> Result<()> {
        if self.is_closed() {
            return Err(Error::ConnectionClosed.into());
        }

        let json = serde_json::to_string(&message)?;

        if let Some(max_size) = self.config.max_message_size {
            if json.len() > max_size {
                return Err(Error::Transport(format!(
                    "Message too large: {} bytes, max: {} bytes",
                    json.len(),
                    max_size
                )));
            }
        }

        let send_future = async {
            let response = self.client
                .post(self.url.clone())
                .header("Content-Type", "application/json")
                .body(json)
                .send()
                .await
                .map_err(|e| Error::Transport(format!("HTTP request failed: {}", e)))?;

            if !response.status().is_success() {
                return Err(Error::Transport(format!(
                    "HTTP request failed with status: {}",
                    response.status()
                )));
            }

            // For HTTP transport, we might receive a response immediately
            let response_text = response.text().await
                .map_err(|e| Error::Transport(format!("Failed to read response: {}", e)))?;

            if !response_text.is_empty() {
                let response_message: JsonRpcMessage = serde_json::from_str(&response_text)?;
                if self.sender.send(response_message).is_err() {
                    return Err(Error::ConnectionClosed);
                }
            }

            Ok(())
        };

        if let Some(timeout) = self.config.write_timeout {
            tokio::time::timeout(timeout, send_future)
                .await
                .map_err(|_| Error::Timeout("Send timeout".to_string()))?
        } else {
            send_future.await
        }
    }

    async fn receive(&mut self) -> Result<Option<JsonRpcMessage>> {
        if self.is_closed() {
            return Ok(None);
        }

        let receive_future = async {
            match self.receiver.recv().await {
                Some(message) => Ok(Some(message)),
                None => {
                    self.closed.store(true, std::sync::atomic::Ordering::SeqCst);
                    Ok(None)
                }
            }
        };

        if let Some(timeout) = self.config.read_timeout {
            tokio::time::timeout(timeout, receive_future)
                .await
                .map_err(|_| Error::Timeout("Receive timeout".to_string()))?
        } else {
            receive_future.await
        }
    }

    async fn close(&mut self) -> Result<()> {
        self.closed.store(true, std::sync::atomic::Ordering::SeqCst);
        self.receiver.close();
        Ok(())
    }

    fn is_closed(&self) -> bool {
        self.closed.load(std::sync::atomic::Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_transport_creation() -> Result<()> {
        let transport = HttpTransport::new("http://localhost:8080")?;
        assert!(!transport.is_closed());
        Ok(())
    }

    #[tokio::test]
    async fn test_http_transport_with_config() -> Result<()> {
        let config = TransportConfig::new()
            .with_max_message_size(1024)
            .with_read_timeout(std::time::Duration::from_secs(5));

        let transport = HttpTransport::with_config("http://localhost:8080", config)?;
        assert!(!transport.is_closed());
        Ok(())
    }

    #[test]
    fn test_invalid_url() {
        let result = HttpTransport::new("not-a-url");
        assert!(result.is_err());
    }
}