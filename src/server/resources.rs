use crate::protocol::{
    Resource, ResourceContents, ResourceTemplate, McpError,
};
use crate::Result;
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait ResourceProvider: Send + Sync {
    async fn list_resources(&self) -> Result<Vec<Resource>>;
    async fn read_resource(&self, uri: &str) -> Result<Vec<ResourceContents>>;

    // Optional: support for resource templates
    async fn list_resource_templates(&self) -> Result<Vec<ResourceTemplate>> {
        Ok(Vec::new())
    }

    // Optional: support for subscription-based updates
    async fn subscribe(&self, _uri: &str) -> Result<()> {
        Err(crate::protocol::GlyphError::JsonRpc(
            "Resource subscriptions not supported".to_string()
        ).into())
    }

    async fn unsubscribe(&self, _uri: &str) -> Result<()> {
        Err(crate::protocol::GlyphError::JsonRpc(
            "Resource subscriptions not supported".to_string()
        ).into())
    }
}

pub struct ResourceRegistry {
    providers: Vec<Box<dyn ResourceProvider>>,
    subscriptions: HashMap<String, Vec<String>>, // uri -> list of subscriber IDs
}

impl ResourceRegistry {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            subscriptions: HashMap::new(),
        }
    }

    pub async fn register(&mut self, provider: Box<dyn ResourceProvider>) -> Result<()> {
        self.providers.push(provider);
        Ok(())
    }

    pub async fn list_resources(&self) -> Result<Vec<Resource>> {
        let mut all_resources = Vec::new();

        for provider in &self.providers {
            let resources = provider.list_resources().await?;
            all_resources.extend(resources);
        }

        // Sort by URI for consistent ordering
        all_resources.sort_by(|a, b| a.uri.cmp(&b.uri));
        Ok(all_resources)
    }

    pub async fn list_resource_templates(&self) -> Result<Vec<ResourceTemplate>> {
        let mut all_templates = Vec::new();

        for provider in &self.providers {
            let templates = provider.list_resource_templates().await?;
            all_templates.extend(templates);
        }

        // Sort by URI template for consistent ordering
        all_templates.sort_by(|a, b| a.uri_template.cmp(&b.uri_template));
        Ok(all_templates)
    }

    pub async fn read_resource(&self, uri: &str) -> Result<Vec<ResourceContents>> {
        // Try each provider until one can handle the URI
        for provider in &self.providers {
            match provider.read_resource(uri).await {
                Ok(contents) => return Ok(contents),
                Err(_) => continue, // Try next provider
            }
        }

        Err(McpError::resource_not_found(uri).into())
    }

    pub async fn subscribe(&mut self, uri: &str, subscriber_id: &str) -> Result<()> {
        // Try to subscribe with providers
        let mut success = false;
        for provider in &self.providers {
            if provider.subscribe(uri).await.is_ok() {
                success = true;
                break;
            }
        }

        if !success {
            return Err(crate::protocol::GlyphError::JsonRpc(
                format!("No provider supports subscription to URI: {}", uri)
            ).into());
        }

        // Track subscription
        self.subscriptions
            .entry(uri.to_string())
            .or_insert_with(Vec::new)
            .push(subscriber_id.to_string());

        Ok(())
    }

    pub async fn unsubscribe(&mut self, uri: &str, subscriber_id: &str) -> Result<()> {
        // Remove from tracking
        if let Some(subscribers) = self.subscriptions.get_mut(uri) {
            subscribers.retain(|id| id != subscriber_id);
            if subscribers.is_empty() {
                self.subscriptions.remove(uri);

                // Unsubscribe from providers
                for provider in &self.providers {
                    let _ = provider.unsubscribe(uri).await; // Ignore errors
                }
            }
        }

        Ok(())
    }

    pub fn get_subscribers(&self, uri: &str) -> Vec<String> {
        self.subscriptions
            .get(uri)
            .cloned()
            .unwrap_or_default()
    }

    pub fn provider_count(&self) -> usize {
        self.providers.len()
    }
}

impl Default for ResourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Built-in resource providers
pub struct FileSystemResourceProvider {
    base_path: std::path::PathBuf,
    allowed_extensions: Option<Vec<String>>,
}

impl FileSystemResourceProvider {
    pub fn new(base_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
            allowed_extensions: None,
        }
    }

    pub fn with_allowed_extensions(mut self, extensions: Vec<String>) -> Self {
        self.allowed_extensions = Some(extensions);
        self
    }

    fn is_allowed_file(&self, path: &std::path::Path) -> bool {
        if let Some(ref allowed) = self.allowed_extensions {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                return allowed.iter().any(|a| a == ext);
            }
            return false;
        }
        true
    }

    fn path_to_uri(&self, path: &std::path::Path) -> String {
        format!("file://{}", path.display())
    }

    fn uri_to_path(&self, uri: &str) -> Option<std::path::PathBuf> {
        if let Some(path_str) = uri.strip_prefix("file://") {
            let path = std::path::Path::new(path_str);
            // Ensure the path is within our base path
            if path.starts_with(&self.base_path) {
                return Some(path.to_path_buf());
            }
        }
        None
    }
}

#[async_trait]
impl ResourceProvider for FileSystemResourceProvider {
    async fn list_resources(&self) -> Result<Vec<Resource>> {
        let mut resources = Vec::new();

        fn visit_dir(
            dir: &std::path::Path,
            provider: &FileSystemResourceProvider,
            resources: &mut Vec<Resource>,
        ) -> std::io::Result<()> {
            if dir.is_dir() {
                for entry in std::fs::read_dir(dir)? {
                    let entry = entry?;
                    let path = entry.path();

                    if path.is_dir() {
                        visit_dir(&path, provider, resources)?;
                    } else if provider.is_allowed_file(&path) {
                        let uri = provider.path_to_uri(&path);
                        let name = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("unknown")
                            .to_string();

                        resources.push(Resource::new(uri, name));
                    }
                }
            }
            Ok(())
        }

        visit_dir(&self.base_path, self, &mut resources)
            .map_err(|e| crate::protocol::GlyphError::Io(e))?;

        Ok(resources)
    }

    async fn read_resource(&self, uri: &str) -> Result<Vec<ResourceContents>> {
        let path = self.uri_to_path(uri)
            .ok_or_else(|| McpError::resource_not_found(uri))?;

        if !path.exists() {
            return Err(McpError::resource_not_found(uri).into());
        }

        let contents = tokio::fs::read_to_string(&path).await
            .map_err(|e| crate::protocol::GlyphError::Io(e))?;

        // Determine MIME type based on file extension
        let mime_type = match path.extension().and_then(|e| e.to_str()) {
            Some("txt") => Some("text/plain".to_string()),
            Some("md") => Some("text/markdown".to_string()),
            Some("json") => Some("application/json".to_string()),
            Some("xml") => Some("application/xml".to_string()),
            Some("html") => Some("text/html".to_string()),
            Some("js") => Some("application/javascript".to_string()),
            Some("css") => Some("text/css".to_string()),
            Some("rs") => Some("text/rust".to_string()),
            _ => None,
        };

        Ok(vec![ResourceContents::text_with_mime_type(
            uri,
            contents,
            mime_type.unwrap_or_else(|| "text/plain".to_string()),
        )])
    }
}

pub struct MemoryResourceProvider {
    resources: HashMap<String, (String, Option<String>)>, // uri -> (content, mime_type)
}

impl MemoryResourceProvider {
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
        }
    }

    pub fn add_resource(
        &mut self,
        uri: String,
        content: String,
        mime_type: Option<String>,
    ) {
        self.resources.insert(uri, (content, mime_type));
    }

    pub fn remove_resource(&mut self, uri: &str) -> bool {
        self.resources.remove(uri).is_some()
    }

    pub fn update_resource(
        &mut self,
        uri: &str,
        content: String,
        mime_type: Option<String>,
    ) -> bool {
        if self.resources.contains_key(uri) {
            self.resources.insert(uri.to_string(), (content, mime_type));
            true
        } else {
            false
        }
    }
}

impl Default for MemoryResourceProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ResourceProvider for MemoryResourceProvider {
    async fn list_resources(&self) -> Result<Vec<Resource>> {
        let mut resources = Vec::new();

        for (uri, (_, mime_type)) in &self.resources {
            let name = uri.split('/').last().unwrap_or(uri).to_string();
            let mut resource = Resource::new(uri.clone(), name);

            if let Some(mime_type) = mime_type {
                resource = resource.with_mime_type(mime_type.clone());
            }

            resources.push(resource);
        }

        resources.sort_by(|a, b| a.uri.cmp(&b.uri));
        Ok(resources)
    }

    async fn read_resource(&self, uri: &str) -> Result<Vec<ResourceContents>> {
        let (content, mime_type) = self.resources.get(uri)
            .ok_or_else(|| McpError::resource_not_found(uri))?;

        Ok(vec![ResourceContents::text_with_mime_type(
            uri,
            content.clone(),
            mime_type.clone().unwrap_or_else(|| "text/plain".to_string()),
        )])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_resource_registry() {
        let mut registry = ResourceRegistry::new();

        // Add memory provider
        let mut memory_provider = MemoryResourceProvider::new();
        memory_provider.add_resource(
            "memory://test.txt".to_string(),
            "Test content".to_string(),
            Some("text/plain".to_string()),
        );

        registry.register(Box::new(memory_provider)).await.unwrap();

        // Test listing resources
        let resources = registry.list_resources().await.unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].uri, "memory://test.txt");

        // Test reading resource
        let contents = registry.read_resource("memory://test.txt").await.unwrap();
        assert_eq!(contents.len(), 1);

        if let ResourceContents::Text { text, .. } = &contents[0] {
            assert_eq!(text, "Test content");
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_memory_resource_provider() {
        let mut provider = MemoryResourceProvider::new();

        provider.add_resource(
            "test://example.json".to_string(),
            r#"{"key": "value"}"#.to_string(),
            Some("application/json".to_string()),
        );

        let resources = provider.list_resources().await.unwrap();
        assert_eq!(resources.len(), 1);

        let contents = provider.read_resource("test://example.json").await.unwrap();
        assert_eq!(contents.len(), 1);
    }
}