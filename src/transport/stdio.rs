use async_trait::async_trait;
use crate::protocol::JsonRpcMessage;
use crate::Error;
use crate::Result;
use crate::transport::{Transport, TransportConfig};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::Mutex;
use std::sync::Arc;

#[derive(Debug)]
pub struct StdioTransport {
    reader: Arc<Mutex<BufReader<tokio::io::Stdin>>>,
    writer: Arc<Mutex<BufWriter<tokio::io::Stdout>>>,
    config: TransportConfig,
    closed: Arc<std::sync::atomic::AtomicBool>,
}

impl StdioTransport {
    pub fn new() -> Self {
        Self::with_config(TransportConfig::default())
    }

    pub fn with_config(config: TransportConfig) -> Self {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        Self {
            reader: Arc::new(Mutex::new(BufReader::new(stdin))),
            writer: Arc::new(Mutex::new(BufWriter::new(stdout))),
            config,
            closed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send(&mut self, message: JsonRpcMessage) -> Result<()> {
        if self.is_closed() {
            return Err(Error::ConnectionClosed);
        }

        let json = serde_json::to_string(&message)?;
        let mut line = json;
        line.push('\n');

        let write_future = async {
            let mut writer = self.writer.lock().await;
            writer.write_all(line.as_bytes()).await?;
            writer.flush().await?;
            Ok::<(), Error>(())
        };

        if let Some(timeout) = self.config.write_timeout {
            tokio::time::timeout(timeout, write_future)
                .await
                .map_err(|_| Error::Timeout("Write timeout".to_string()))?
        } else {
            write_future.await
        }
    }

    async fn receive(&mut self) -> Result<Option<JsonRpcMessage>> {
        if self.is_closed() {
            return Ok(None);
        }

        let read_future = async {
            let mut reader = self.reader.lock().await;
            let mut line = String::new();

            match reader.read_line(&mut line).await? {
                0 => {
                    // EOF reached
                    self.closed.store(true, std::sync::atomic::Ordering::SeqCst);
                    Ok(None)
                }
                _ => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        return Ok(None);
                    }

                    if let Some(max_size) = self.config.max_message_size {
                        if trimmed.len() > max_size {
                            return Err(Error::Transport(format!(
                                "Message too large: {} bytes, max: {} bytes",
                                trimmed.len(),
                                max_size
                            )));
                        }
                    }

                    let message: JsonRpcMessage = serde_json::from_str(trimmed)?;
                    Ok(Some(message))
                }
            }
        };

        if let Some(timeout) = self.config.read_timeout {
            tokio::time::timeout(timeout, read_future)
                .await
                .map_err(|_| Error::Timeout("Read timeout".to_string()))?
        } else {
            read_future.await
        }
    }

    async fn close(&mut self) -> Result<()> {
        self.closed.store(true, std::sync::atomic::Ordering::SeqCst);

        // Flush any remaining data
        let mut writer = self.writer.lock().await;
        writer.flush().await?;

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
    async fn test_stdio_transport_creation() {
        let transport = StdioTransport::new();
        assert!(!transport.is_closed());
    }

    #[tokio::test]
    async fn test_stdio_transport_with_config() {
        let config = TransportConfig::new()
            .with_max_message_size(1024)
            .with_read_timeout(std::time::Duration::from_secs(5));

        let transport = StdioTransport::with_config(config);
        assert!(!transport.is_closed());
    }
}