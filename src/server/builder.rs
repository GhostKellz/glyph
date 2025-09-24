use crate::protocol::{Implementation, ServerCapabilities, ToolsCapability, ResourcesCapability, PromptsCapability};
use crate::server::{Server, SessionManager, ToolRegistry, ResourceRegistry, PromptRegistry};
use crate::transport::{StdioTransport, WebSocketServer, TransportConfig};
use crate::Result;

#[derive(Debug)]
pub struct ServerBuilder {
    server_info: Option<Implementation>,
    capabilities: ServerCapabilities,
    transport_config: TransportConfig,
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self {
            server_info: None,
            capabilities: ServerCapabilities::default(),
            transport_config: TransportConfig::default(),
        }
    }

    pub fn with_server_info(mut self, name: impl Into<String>, version: impl Into<String>) -> Self {
        self.server_info = Some(Implementation::new(name, version));
        self
    }

    pub fn with_capabilities(mut self, capabilities: ServerCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn with_tools(mut self) -> Self {
        self.capabilities = self.capabilities.with_tools(ToolsCapability::new());
        self
    }

    pub fn with_tool_list_changes(mut self) -> Self {
        self.capabilities = self.capabilities.with_tools(
            ToolsCapability::new().with_list_changed(true)
        );
        self
    }

    pub fn with_resources(mut self) -> Self {
        self.capabilities = self.capabilities.with_resources(ResourcesCapability::new());
        self
    }

    pub fn with_resource_subscriptions(mut self) -> Self {
        self.capabilities = self.capabilities.with_resources(
            ResourcesCapability::new()
                .with_subscribe(true)
                .with_list_changed(true)
        );
        self
    }

    pub fn with_prompts(mut self) -> Self {
        self.capabilities = self.capabilities.with_prompts(PromptsCapability::new());
        self
    }

    pub fn with_prompt_list_changes(mut self) -> Self {
        self.capabilities = self.capabilities.with_prompts(
            PromptsCapability::new().with_list_changed(true)
        );
        self
    }

    pub fn with_transport_config(mut self, config: TransportConfig) -> Self {
        self.transport_config = config;
        self
    }

    pub fn build(&self) -> Server {
        let server_info = self.server_info.clone().unwrap_or_else(|| {
            Implementation::new("glyph-server", env!("CARGO_PKG_VERSION"))
        });

        let session_manager = SessionManager::new();
        let tool_registry = ToolRegistry::new();
        let resource_registry = ResourceRegistry::new();
        let prompt_registry = PromptRegistry::new();

        Server::new(
            self.capabilities.clone(),
            server_info,
            session_manager,
            tool_registry,
            resource_registry,
            prompt_registry,
        )
    }

    // Convenience methods for common setups
    pub fn for_stdio(&self) -> ServerWithStdio {
        ServerWithStdio {
            server: self.build(),
            config: self.transport_config.clone(),
        }
    }

    pub async fn for_websocket(&self, addr: &str) -> Result<ServerWithWebSocket> {
        let server = self.build();
        let ws_server = WebSocketServer::bind_with_config(addr, self.transport_config.clone()).await?;

        Ok(ServerWithWebSocket {
            server,
            ws_server,
        })
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// Convenience wrappers
pub struct ServerWithStdio {
    server: Server,
    config: TransportConfig,
}

impl ServerWithStdio {
    pub async fn run(self) -> Result<()> {
        let transport = StdioTransport::with_config(self.config);
        self.server.run_with_transport(transport).await
    }

    pub fn server(&self) -> &Server {
        &self.server
    }

    pub fn into_server(self) -> Server {
        self.server
    }
}

pub struct ServerWithWebSocket {
    server: Server,
    ws_server: WebSocketServer,
}

impl ServerWithWebSocket {
    pub async fn run(self) -> Result<()> {
        self.server.run_with_server(self.ws_server).await
    }

    pub fn server(&self) -> &Server {
        &self.server
    }

    pub fn into_server(self) -> (Server, WebSocketServer) {
        (self.server, self.ws_server)
    }

    pub fn local_addr(&self) -> Result<std::net::SocketAddr> {
        self.ws_server.local_addr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_builder() {
        let server = ServerBuilder::new()
            .with_server_info("test-server", "1.0.0")
            .with_tools()
            .with_resources()
            .with_prompts()
            .build();

        assert!(server.capabilities().supports_tools());
        assert!(server.capabilities().supports_resources());
        assert!(server.capabilities().supports_prompts());
        assert_eq!(server.server_info().name, "test-server");
        assert_eq!(server.server_info().version, "1.0.0");
    }

    #[test]
    fn test_default_server_builder() {
        let server = ServerBuilder::default().build();
        assert_eq!(server.server_info().name, "glyph-server");
    }
}