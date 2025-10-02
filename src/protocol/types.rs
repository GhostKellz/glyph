use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Number(i64),
    String(String),
}

impl RequestId {
    pub fn new_string(s: impl Into<String>) -> Self {
        Self::String(s.into())
    }

    pub fn new_number(n: i64) -> Self {
        Self::Number(n)
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<i64> for RequestId {
    fn from(n: i64) -> Self {
        Self::Number(n)
    }
}

impl From<i32> for RequestId {
    fn from(n: i32) -> Self {
        Self::Number(n as i64)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Implementation {
    pub name: String,
    pub version: String,
}

impl Implementation {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }

    pub fn glyph() -> Self {
        Self::new("glyph", env!("CARGO_PKG_VERSION"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Content {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    #[serde(rename = "resource")]
    Resource {
        #[serde(rename = "resourceUri")]
        resource_uri: String,
        text: Option<String>,
        #[serde(rename = "mimeType")]
        mime_type: Option<String>,
    },
}

impl Content {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Image {
            data: data.into(),
            mime_type: mime_type.into(),
        }
    }

    pub fn resource(uri: impl Into<String>) -> Self {
        Self::Resource {
            resource_uri: uri.into(),
            text: None,
            mime_type: None,
        }
    }

    pub fn resource_with_text(
        uri: impl Into<String>,
        text: impl Into<String>,
        mime_type: Option<String>,
    ) -> Self {
        Self::Resource {
            resource_uri: uri.into(),
            text: Some(text.into()),
            mime_type,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: ToolInputSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: Option<HashMap<String, serde_json::Value>>,
    pub required: Option<Vec<String>>,
    #[serde(flatten)]
    pub additional: HashMap<String, serde_json::Value>,
}

impl ToolInputSchema {
    pub fn object() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: Some(HashMap::new()),
            required: None,
            additional: HashMap::new(),
        }
    }

    pub fn with_properties(mut self, properties: HashMap<String, serde_json::Value>) -> Self {
        self.properties = Some(properties);
        self
    }

    pub fn with_required(mut self, required: Vec<String>) -> Self {
        self.required = Some(required);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

impl Resource {
    pub fn new(uri: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: name.into(),
            description: None,
            mime_type: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = Some(mime_type.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTemplate {
    #[serde(rename = "uriTemplate")]
    pub uri_template: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub name: String,
    pub description: Option<String>,
    pub arguments: Option<Vec<PromptArgument>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: PromptRole,
    pub content: Content,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PromptRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification<T> {
    pub method: String,
    pub params: Option<T>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSubscription {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceListChanged;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolListChanged;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptListChanged;