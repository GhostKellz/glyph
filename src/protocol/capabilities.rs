use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<SamplingCapability>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingCapability;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptsCapability {
    #[serde(rename = "listChanged")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

impl PromptsCapability {
    pub fn new() -> Self {
        Self { list_changed: None }
    }

    pub fn with_list_changed(mut self, list_changed: bool) -> Self {
        self.list_changed = Some(list_changed);
        self
    }
}

impl Default for PromptsCapability {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapability {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscribe: Option<bool>,
    #[serde(rename = "listChanged")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

impl ResourcesCapability {
    pub fn new() -> Self {
        Self {
            subscribe: None,
            list_changed: None,
        }
    }

    pub fn with_subscribe(mut self, subscribe: bool) -> Self {
        self.subscribe = Some(subscribe);
        self
    }

    pub fn with_list_changed(mut self, list_changed: bool) -> Self {
        self.list_changed = Some(list_changed);
        self
    }
}

impl Default for ResourcesCapability {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsCapability {
    #[serde(rename = "listChanged")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub list_changed: Option<bool>,
}

impl ToolsCapability {
    pub fn new() -> Self {
        Self { list_changed: None }
    }

    pub fn with_list_changed(mut self, list_changed: bool) -> Self {
        self.list_changed = Some(list_changed);
        self
    }
}

impl Default for ToolsCapability {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientCapabilities {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_sampling(mut self) -> Self {
        self.sampling = Some(SamplingCapability);
        self
    }

    pub fn with_experimental(mut self, experimental: serde_json::Value) -> Self {
        self.experimental = Some(experimental);
        self
    }
}

impl ServerCapabilities {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_tools(mut self, tools: ToolsCapability) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn with_resources(mut self, resources: ResourcesCapability) -> Self {
        self.resources = Some(resources);
        self
    }

    pub fn with_prompts(mut self, prompts: PromptsCapability) -> Self {
        self.prompts = Some(prompts);
        self
    }

    pub fn with_logging(mut self, logging: serde_json::Value) -> Self {
        self.logging = Some(logging);
        self
    }

    pub fn with_experimental(mut self, experimental: serde_json::Value) -> Self {
        self.experimental = Some(experimental);
        self
    }

    pub fn supports_tools(&self) -> bool {
        self.tools.is_some()
    }

    pub fn supports_resources(&self) -> bool {
        self.resources.is_some()
    }

    pub fn supports_prompts(&self) -> bool {
        self.prompts.is_some()
    }

    pub fn supports_resource_subscriptions(&self) -> bool {
        self.resources
            .as_ref()
            .and_then(|r| r.subscribe)
            .unwrap_or(false)
    }

    pub fn supports_tool_list_changes(&self) -> bool {
        self.tools
            .as_ref()
            .and_then(|t| t.list_changed)
            .unwrap_or(false)
    }

    pub fn supports_resource_list_changes(&self) -> bool {
        self.resources
            .as_ref()
            .and_then(|r| r.list_changed)
            .unwrap_or(false)
    }

    pub fn supports_prompt_list_changes(&self) -> bool {
        self.prompts
            .as_ref()
            .and_then(|p| p.list_changed)
            .unwrap_or(false)
    }
}