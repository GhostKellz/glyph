use crate::protocol::{
    Prompt, GetPromptRequest, GetPromptResult, ListPromptsRequest, ListPromptsResult, RequestId,
};
use crate::client::{Connection, ResponseWaiter};
use crate::{Result, protocol::{JsonRpcMessage, JsonRpcRequest}};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

#[derive(Debug, Clone)]
pub struct PromptClient {
    connection: Arc<Mutex<Connection>>,
    pending_requests: Arc<RwLock<HashMap<RequestId, ResponseWaiter>>>,
    request_counter: Arc<std::sync::atomic::AtomicU64>,
}

impl PromptClient {
    pub fn new(
        connection: Arc<Mutex<Connection>>,
        pending_requests: Arc<RwLock<HashMap<RequestId, ResponseWaiter>>>,
    ) -> Self {
        Self {
            connection,
            pending_requests,
            request_counter: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    pub async fn list_prompts(&self, cursor: Option<String>) -> Result<ListPromptsResult> {
        let request = ListPromptsRequest { cursor };
        self.send_request("prompts/list", Some(request)).await
    }

    pub async fn get_prompt(
        &self,
        name: impl Into<String>,
        arguments: Option<HashMap<String, String>>,
    ) -> Result<GetPromptResult> {
        let request = GetPromptRequest {
            name: name.into(),
            arguments,
        };
        self.send_request("prompts/get", Some(request)).await
    }

    pub async fn render_prompt(
        &self,
        name: impl Into<String>,
        arguments: HashMap<String, String>,
    ) -> Result<String> {
        let result = self.get_prompt(name, Some(arguments)).await?;

        // Combine all messages into a single string
        let mut rendered = String::new();

        if let Some(description) = result.description {
            rendered.push_str(&format!("# {}\n\n", description));
        }

        for message in result.messages {
            match message.role {
                crate::protocol::PromptRole::System => rendered.push_str("**System**: "),
                crate::protocol::PromptRole::User => rendered.push_str("**User**: "),
                crate::protocol::PromptRole::Assistant => rendered.push_str("**Assistant**: "),
            }

            match message.content {
                crate::protocol::Content::Text { text } => {
                    rendered.push_str(&text);
                    rendered.push('\n');
                }
                crate::protocol::Content::Image { .. } => {
                    rendered.push_str("[Image content]");
                    rendered.push('\n');
                }
                crate::protocol::Content::Resource { resource_uri, text, .. } => {
                    if let Some(text) = text {
                        rendered.push_str(&text);
                    } else {
                        rendered.push_str(&format!("[Resource: {}]", resource_uri));
                    }
                    rendered.push('\n');
                }
            }
            rendered.push('\n');
        }

        Ok(rendered.trim().to_string())
    }

    async fn send_request<T, R>(&self, method: &str, params: Option<T>) -> Result<R>
    where
        T: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let id = RequestId::Number(
            self.request_counter
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst) as i64
        );

        let params = match params {
            Some(p) => Some(serde_json::to_value(p)?),
            None => None,
        };

        let request = JsonRpcRequest::new(id.clone(), method, params);

        // Create response waiter
        let (tx, rx) = tokio::sync::oneshot::channel();
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(id.clone(), tx);
        }

        // Send request
        {
            let mut conn = self.connection.lock().await;
            conn.send(JsonRpcMessage::Request(request)).await?;
        }

        // Wait for response
        let response_value = rx.await
            .map_err(|_| crate::protocol::GlyphError::JsonRpc("Request cancelled".to_string()))??;

        // Deserialize response
        Ok(serde_json::from_value::<R>(response_value)?)
    }
}

// Convenience wrapper for a specific prompt
#[derive(Debug)]
pub struct PromptHandle {
    client: PromptClient,
    name: String,
}

impl PromptHandle {
    pub fn new(client: PromptClient, name: impl Into<String>) -> Self {
        Self {
            client,
            name: name.into(),
        }
    }

    pub async fn get(&self, arguments: Option<HashMap<String, String>>) -> Result<GetPromptResult> {
        self.client.get_prompt(&self.name, arguments).await
    }

    pub async fn render(&self, arguments: HashMap<String, String>) -> Result<String> {
        self.client.render_prompt(&self.name, arguments).await
    }

    pub async fn render_simple(&self) -> Result<String> {
        self.render(HashMap::new()).await
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

// Prompt discovery and caching
#[derive(Debug)]
pub struct PromptRegistry {
    client: PromptClient,
    prompts: Arc<RwLock<Vec<Prompt>>>,
    last_updated: Arc<RwLock<Option<std::time::Instant>>>,
    cache_duration: std::time::Duration,
}

impl PromptRegistry {
    pub fn new(client: PromptClient) -> Self {
        Self {
            client,
            prompts: Arc::new(RwLock::new(Vec::new())),
            last_updated: Arc::new(RwLock::new(None)),
            cache_duration: std::time::Duration::from_secs(300), // 5 minutes
        }
    }

    pub fn with_cache_duration(mut self, duration: std::time::Duration) -> Self {
        self.cache_duration = duration;
        self
    }

    pub async fn refresh(&self) -> Result<()> {
        let result = self.client.list_prompts(None).await?;

        {
            let mut prompts = self.prompts.write().await;
            *prompts = result.prompts;
        }

        {
            let mut last_updated = self.last_updated.write().await;
            *last_updated = Some(std::time::Instant::now());
        }

        Ok(())
    }

    pub async fn get_prompts(&self) -> Result<Vec<Prompt>> {
        let should_refresh = {
            let last_updated = self.last_updated.read().await;
            match *last_updated {
                Some(time) => time.elapsed() > self.cache_duration,
                None => true,
            }
        };

        if should_refresh {
            self.refresh().await?;
        }

        let prompts = self.prompts.read().await;
        Ok(prompts.clone())
    }

    pub async fn get_prompt(&self, name: &str) -> Result<Option<Prompt>> {
        let prompts = self.get_prompts().await?;
        Ok(prompts.into_iter().find(|p| p.name == name))
    }

    pub async fn get_prompt_handle(&self, name: &str) -> Result<Option<PromptHandle>> {
        if self.get_prompt(name).await?.is_some() {
            Ok(Some(PromptHandle::new(self.client.clone(), name)))
        } else {
            Ok(None)
        }
    }

    pub async fn prompt_names(&self) -> Result<Vec<String>> {
        let prompts = self.get_prompts().await?;
        Ok(prompts.into_iter().map(|p| p.name).collect())
    }

    pub async fn find_prompts_with_arguments(&self) -> Result<Vec<Prompt>> {
        let prompts = self.get_prompts().await?;
        Ok(prompts.into_iter()
            .filter(|p| p.arguments.as_ref().map_or(false, |args| !args.is_empty()))
            .collect())
    }
}

// Prompt builder for complex prompts
#[derive(Debug)]
pub struct PromptBuilder {
    name: String,
    arguments: HashMap<String, String>,
}

impl PromptBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: HashMap::new(),
        }
    }

    pub fn with_argument(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.arguments.insert(key.into(), value.into());
        self
    }

    pub fn with_arguments(mut self, arguments: HashMap<String, String>) -> Self {
        self.arguments.extend(arguments);
        self
    }

    pub async fn execute(self, client: &PromptClient) -> Result<GetPromptResult> {
        client.get_prompt(self.name, Some(self.arguments)).await
    }

    pub async fn render(self, client: &PromptClient) -> Result<String> {
        client.render_prompt(self.name, self.arguments).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::StdioTransport;

    #[tokio::test]
    async fn test_prompt_handle_creation() {
        let transport = StdioTransport::new();
        let connection = Arc::new(Mutex::new(crate::client::Connection::new(Box::new(transport))));
        let pending_requests = Arc::new(RwLock::new(HashMap::new()));

        let client = PromptClient::new(connection, pending_requests);
        let handle = PromptHandle::new(client, "test_prompt");

        assert_eq!(handle.name(), "test_prompt");
    }

    #[tokio::test]
    async fn test_prompt_builder() {
        let builder = PromptBuilder::new("code_review")
            .with_argument("language", "rust")
            .with_argument("code", "fn main() { println!(\"Hello\"); }");

        assert_eq!(builder.name, "code_review");
        assert_eq!(builder.arguments.get("language"), Some(&"rust".to_string()));
        assert_eq!(builder.arguments.get("code"), Some(&"fn main() { println!(\"Hello\"); }".to_string()));
    }

    #[tokio::test]
    async fn test_prompt_registry() {
        let transport = StdioTransport::new();
        let connection = Arc::new(Mutex::new(crate::client::Connection::new(Box::new(transport))));
        let pending_requests = Arc::new(RwLock::new(HashMap::new()));

        let client = PromptClient::new(connection, pending_requests);
        let registry = PromptRegistry::new(client)
            .with_cache_duration(std::time::Duration::from_secs(60));

        assert_eq!(registry.cache_duration, std::time::Duration::from_secs(60));
    }
}