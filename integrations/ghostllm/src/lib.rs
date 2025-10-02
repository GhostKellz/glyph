/// GhostLLM Integration for Glyph MCP
///
/// Exposes OpenAI, Anthropic, and Gemini providers as MCP tools through GhostLLM proxy.
/// Includes cost tracking, rate limiting, and auth alignment.

use glyph::server::{Tool, ToolContext};
use glyph::protocol::{CallToolResult, Content, ToolInputSchema};
use glyph::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use async_trait::async_trait;
use reqwest::Client as HttpClient;

// ============================================================================
// Provider Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    OpenAI,
    Anthropic,
    Gemini,
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Provider::OpenAI => write!(f, "openai"),
            Provider::Anthropic => write!(f, "anthropic"),
            Provider::Gemini => write!(f, "gemini"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub id: String,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ============================================================================
// Cost Tracking
// ============================================================================

pub struct CostCalculator;

impl CostCalculator {
    /// Calculate cost in USD based on provider and model
    pub fn calculate(provider: &Provider, model: &str, usage: &Usage) -> f64 {
        let (prompt_rate, completion_rate) = Self::get_rates(provider, model);

        let prompt_cost = (usage.prompt_tokens as f64 / 1_000_000.0) * prompt_rate;
        let completion_cost = (usage.completion_tokens as f64 / 1_000_000.0) * completion_rate;

        prompt_cost + completion_cost
    }

    fn get_rates(provider: &Provider, model: &str) -> (f64, f64) {
        match (provider, model) {
            // OpenAI rates (per 1M tokens)
            (Provider::OpenAI, "gpt-4") => (30.0, 60.0),
            (Provider::OpenAI, "gpt-4-turbo") => (10.0, 30.0),
            (Provider::OpenAI, "gpt-3.5-turbo") => (0.5, 1.5),

            // Anthropic rates
            (Provider::Anthropic, "claude-3-opus") => (15.0, 75.0),
            (Provider::Anthropic, "claude-3-sonnet") => (3.0, 15.0),
            (Provider::Anthropic, "claude-3-haiku") => (0.25, 1.25),

            // Gemini rates
            (Provider::Gemini, "gemini-pro") => (0.5, 1.5),
            (Provider::Gemini, "gemini-ultra") => (10.0, 30.0),

            // Default fallback
            _ => (1.0, 2.0),
        }
    }
}

// ============================================================================
// GhostLLM Client
// ============================================================================

pub struct GhostLLMClient {
    http_client: HttpClient,
    base_url: String,
    api_key: String,
}

impl GhostLLMClient {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            http_client: HttpClient::new(),
            base_url: base_url.into(),
            api_key: api_key.into(),
        }
    }

    pub async fn complete(
        &self,
        provider: &Provider,
        request: CompletionRequest,
    ) -> anyhow::Result<CompletionResponse> {
        let url = format!("{}/v1/{}/chat/completions", self.base_url, provider);

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("GhostLLM API error: {}", error_text);
        }

        let completion: CompletionResponse = response.json().await?;
        Ok(completion)
    }
}

// ============================================================================
// Provider Tools
// ============================================================================

/// OpenAI provider tool
pub struct OpenAITool {
    client: GhostLLMClient,
}

impl OpenAITool {
    pub fn new(ghostllm_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            client: GhostLLMClient::new(ghostllm_url, api_key),
        }
    }
}

#[async_trait]
impl Tool for OpenAITool {
    fn name(&self) -> &str {
        "openai_chat"
    }

    fn description(&self) -> Option<&str> {
        Some("Call OpenAI models through GhostLLM proxy with cost tracking")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut props = HashMap::new();
        props.insert("model".to_string(), json!({
            "type": "string",
            "enum": ["gpt-4", "gpt-4-turbo", "gpt-3.5-turbo"],
            "description": "OpenAI model to use"
        }));
        props.insert("messages".to_string(), json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "role": { "type": "string", "enum": ["system", "user", "assistant"] },
                    "content": { "type": "string" }
                },
                "required": ["role", "content"]
            },
            "description": "Conversation messages"
        }));
        props.insert("temperature".to_string(), json!({
            "type": "number",
            "minimum": 0.0,
            "maximum": 2.0,
            "description": "Sampling temperature (optional)"
        }));
        props.insert("max_tokens".to_string(), json!({
            "type": "integer",
            "description": "Maximum tokens to generate (optional)"
        }));

        ToolInputSchema {
            schema_type: "object".to_string(),
            properties: Some(props),
            required: Some(vec!["model".to_string(), "messages".to_string()]),
            additional: HashMap::new(),
        }
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult> {
        let args = args.ok_or_else(|| glyph::Error::Protocol("Missing arguments".into()))?;

        let request: CompletionRequest = serde_json::from_value(args)
            .map_err(|e| glyph::Error::Protocol(format!("Invalid request: {}", e)))?;

        let model = request.model.clone();

        let response = self
            .client
            .complete(&Provider::OpenAI, request)
            .await
            .map_err(|e| glyph::Error::Protocol(e.to_string()))?;

        let cost = CostCalculator::calculate(&Provider::OpenAI, &model, &response.usage);

        let result_text = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(CallToolResult {
            content: vec![Content::text(result_text)],
            is_error: None,
            meta: Some(json!({
                "provider": "openai",
                "model": model,
                "usage": {
                    "prompt_tokens": response.usage.prompt_tokens,
                    "completion_tokens": response.usage.completion_tokens,
                    "total_tokens": response.usage.total_tokens,
                },
                "cost_usd": cost,
                "finish_reason": response.choices.first().map(|c| c.finish_reason.clone()),
            })),
        })
    }
}

/// Anthropic provider tool
pub struct AnthropicTool {
    client: GhostLLMClient,
}

impl AnthropicTool {
    pub fn new(ghostllm_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            client: GhostLLMClient::new(ghostllm_url, api_key),
        }
    }
}

#[async_trait]
impl Tool for AnthropicTool {
    fn name(&self) -> &str {
        "anthropic_chat"
    }

    fn description(&self) -> Option<&str> {
        Some("Call Anthropic Claude models through GhostLLM proxy with cost tracking")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut props = HashMap::new();
        props.insert("model".to_string(), json!({
            "type": "string",
            "enum": ["claude-3-opus", "claude-3-sonnet", "claude-3-haiku"],
            "description": "Anthropic model to use"
        }));
        props.insert("messages".to_string(), json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "role": { "type": "string", "enum": ["user", "assistant"] },
                    "content": { "type": "string" }
                }
            }
        }));
        props.insert("max_tokens".to_string(), json!({
            "type": "integer",
            "default": 4096
        }));

        ToolInputSchema {
            schema_type: "object".to_string(),
            properties: Some(props),
            required: Some(vec!["model".to_string(), "messages".to_string()]),
            additional: HashMap::new(),
        }
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult> {
        let args = args.ok_or_else(|| glyph::Error::Protocol("Missing arguments".into()))?;

        let request: CompletionRequest = serde_json::from_value(args)
            .map_err(|e| glyph::Error::Protocol(format!("Invalid request: {}", e)))?;

        let model = request.model.clone();

        let response = self
            .client
            .complete(&Provider::Anthropic, request)
            .await
            .map_err(|e| glyph::Error::Protocol(e.to_string()))?;

        let cost = CostCalculator::calculate(&Provider::Anthropic, &model, &response.usage);

        let result_text = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(CallToolResult {
            content: vec![Content::text(result_text)],
            is_error: None,
            meta: Some(json!({
                "provider": "anthropic",
                "model": model,
                "usage": response.usage,
                "cost_usd": cost,
            })),
        })
    }
}

/// Google Gemini provider tool
pub struct GeminiTool {
    client: GhostLLMClient,
}

impl GeminiTool {
    pub fn new(ghostllm_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            client: GhostLLMClient::new(ghostllm_url, api_key),
        }
    }
}

#[async_trait]
impl Tool for GeminiTool {
    fn name(&self) -> &str {
        "gemini_chat"
    }

    fn description(&self) -> Option<&str> {
        Some("Call Google Gemini models through GhostLLM proxy with cost tracking")
    }

    fn input_schema(&self) -> ToolInputSchema {
        let mut props = HashMap::new();
        props.insert("model".to_string(), json!({
            "type": "string",
            "enum": ["gemini-pro", "gemini-ultra"],
            "description": "Gemini model to use"
        }));
        props.insert("messages".to_string(), json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "role": { "type": "string" },
                    "content": { "type": "string" }
                }
            }
        }));

        ToolInputSchema {
            schema_type: "object".to_string(),
            properties: Some(props),
            required: Some(vec!["model".to_string(), "messages".to_string()]),
            additional: HashMap::new(),
        }
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult> {
        let args = args.ok_or_else(|| glyph::Error::Protocol("Missing arguments".into()))?;

        let request: CompletionRequest = serde_json::from_value(args)
            .map_err(|e| glyph::Error::Protocol(format!("Invalid request: {}", e)))?;

        let model = request.model.clone();

        let response = self
            .client
            .complete(&Provider::Gemini, request)
            .await
            .map_err(|e| glyph::Error::Protocol(e.to_string()))?;

        let cost = CostCalculator::calculate(&Provider::Gemini, &model, &response.usage);

        let result_text = response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(CallToolResult {
            content: vec![Content::text(result_text)],
            is_error: None,
            meta: Some(json!({
                "provider": "gemini",
                "model": model,
                "usage": response.usage,
                "cost_usd": cost,
            })),
        })
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Register all provider tools with a Glyph server
pub async fn register_all_providers(
    server: &glyph::server::Server,
    ghostllm_url: impl Into<String>,
    api_key: impl Into<String>,
) -> glyph::Result<()> {
    let url = ghostllm_url.into();
    let key = api_key.into();

    server.register_tool(OpenAITool::new(url.clone(), key.clone())).await?;
    server.register_tool(AnthropicTool::new(url.clone(), key.clone())).await?;
    server.register_tool(GeminiTool::new(url, key)).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cost_calculation() {
        let usage = Usage {
            prompt_tokens: 1000,
            completion_tokens: 500,
            total_tokens: 1500,
        };

        let cost = CostCalculator::calculate(&Provider::OpenAI, "gpt-4", &usage);
        assert!(cost > 0.0);
    }

    #[test]
    fn test_provider_display() {
        assert_eq!(Provider::OpenAI.to_string(), "openai");
        assert_eq!(Provider::Anthropic.to_string(), "anthropic");
        assert_eq!(Provider::Gemini.to_string(), "gemini");
    }
}
