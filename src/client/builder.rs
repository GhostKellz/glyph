use crate::protocol::{Implementation, ClientCapabilities};
use crate::transport::{StdioTransport, WebSocketTransport, HttpTransport, TransportConfig};
use crate::client::{Client, Connection};
use crate::Result;

#[derive(Debug)]
pub struct ClientBuilder {
    client_info: Option<Implementation>,
    capabilities: ClientCapabilities,
    transport_config: TransportConfig,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self {
            client_info: None,
            capabilities: ClientCapabilities::default(),
            transport_config: TransportConfig::default(),
        }
    }

    pub fn with_client_info(mut self, name: impl Into<String>, version: impl Into<String>) -> Self {
        self.client_info = Some(Implementation::new(name, version));
        self
    }

    pub fn with_capabilities(mut self, capabilities: ClientCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn with_sampling(mut self) -> Self {
        self.capabilities = self.capabilities.with_sampling();
        self
    }

    pub fn with_experimental(mut self, experimental: serde_json::Value) -> Self {
        self.capabilities = self.capabilities.with_experimental(experimental);
        self
    }

    pub fn with_transport_config(mut self, config: TransportConfig) -> Self {
        self.transport_config = config;
        self
    }

    fn get_client_info(&self) -> Implementation {
        self.client_info.clone().unwrap_or_else(|| {
            Implementation::new("glyph-client", env!("CARGO_PKG_VERSION"))
        })
    }

    pub async fn connect_stdio(self) -> Result<Client> {
        let transport = StdioTransport::with_config(self.transport_config);
        let connection = Connection::new(Box::new(transport));
        let client = Client::new(connection, self.get_client_info(), self.capabilities);
        client.initialize().await?;
        Ok(client)
    }

    pub async fn connect_websocket(self, url: &str) -> Result<Client> {
        let transport = WebSocketTransport::connect_with_config(url, self.transport_config).await?;
        let connection = Connection::new(Box::new(transport));
        let client = Client::new(connection, self.get_client_info(), self.capabilities);
        client.initialize().await?;
        Ok(client)
    }

    pub async fn connect_http(self, url: &str) -> Result<Client> {
        let mut transport = HttpTransport::with_config(url, self.transport_config)?;
        transport.start_sse_listener().await?;
        let connection = Connection::new(Box::new(transport));
        let client = Client::new(connection, self.get_client_info(), self.capabilities);
        client.initialize().await?;
        Ok(client)
    }

    pub async fn connect_with_transport<T: Transport + 'static>(
        self,
        transport: T,
    ) -> Result<Client> {
        let connection = Connection::new(Box::new(transport));
        let client = Client::new(connection, self.get_client_info(), self.capabilities);
        client.initialize().await?;
        Ok(client)
    }

    pub fn build_without_connecting(self) -> ClientConfig {
        ClientConfig {
            client_info: self.get_client_info(),
            capabilities: self.capabilities,
            transport_config: self.transport_config,
        }
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub client_info: Implementation,
    pub capabilities: ClientCapabilities,
    pub transport_config: TransportConfig,
}

impl ClientConfig {
    pub async fn connect_stdio(self) -> Result<Client> {
        let transport = StdioTransport::with_config(self.transport_config);
        let connection = Connection::new(Box::new(transport));
        let client = Client::new(connection, self.client_info, self.capabilities);
        client.initialize().await?;
        Ok(client)
    }

    pub async fn connect_websocket(self, url: &str) -> Result<Client> {
        let transport = WebSocketTransport::connect_with_config(url, self.transport_config).await?;
        let connection = Connection::new(Box::new(transport));
        let client = Client::new(connection, self.client_info, self.capabilities);
        client.initialize().await?;
        Ok(client)
    }

    pub async fn connect_http(self, url: &str) -> Result<Client> {
        let mut transport = HttpTransport::with_config(url, self.transport_config)?;
        transport.start_sse_listener().await?;
        let connection = Connection::new(Box::new(transport));
        let client = Client::new(connection, self.client_info, self.capabilities);
        client.initialize().await?;
        Ok(client)
    }

    pub async fn connect_with_transport<T: Transport + 'static>(
        self,
        transport: T,
    ) -> Result<Client> {
        let connection = Connection::new(Box::new(transport));
        let client = Client::new(connection, self.client_info, self.capabilities);
        client.initialize().await?;
        Ok(client)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_builder() {
        let config = ClientBuilder::new()
            .with_client_info("test-client", "1.0.0")
            .with_sampling()
            .build_without_connecting();

        assert_eq!(config.client_info.name, "test-client");
        assert_eq!(config.client_info.version, "1.0.0");
        assert!(config.capabilities.sampling.is_some());
    }

    #[test]
    fn test_default_client_builder() {
        let config = ClientBuilder::default().build_without_connecting();
        assert_eq!(config.client_info.name, "glyph-client");
    }
}