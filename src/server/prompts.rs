use crate::protocol::{
    Prompt, PromptArgument, PromptMessage, GetPromptResult, McpError,
};
use crate::Result;
use async_trait::async_trait;
use std::collections::HashMap;

#[async_trait]
pub trait PromptProvider: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> Option<&str> {
        None
    }
    fn arguments(&self) -> Vec<PromptArgument> {
        Vec::new()
    }

    async fn get_prompt(&self, arguments: HashMap<String, String>) -> Result<GetPromptResult>;
}

pub struct PromptRegistry {
    prompts: HashMap<String, Box<dyn PromptProvider>>,
}

impl PromptRegistry {
    pub fn new() -> Self {
        Self {
            prompts: HashMap::new(),
        }
    }

    pub async fn register(&mut self, prompt: Box<dyn PromptProvider>) -> Result<()> {
        let name = prompt.name().to_string();

        if self.prompts.contains_key(&name) {
            return Err(crate::Error::protocol(format!("Prompt '{}' is already registered", name)));
        }

        self.prompts.insert(name, prompt);
        Ok(())
    }

    pub async fn list_prompts(&self) -> Result<Vec<Prompt>> {
        let mut prompts = Vec::new();

        for prompt in self.prompts.values() {
            prompts.push(Prompt {
                name: prompt.name().to_string(),
                description: prompt.description().map(|s| s.to_string()),
                arguments: Some(prompt.arguments()),
            });
        }

        prompts.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(prompts)
    }

    pub async fn get_prompt(
        &self,
        name: &str,
        arguments: HashMap<String, String>,
    ) -> Result<GetPromptResult> {
        let prompt = self.prompts.get(name)
            .ok_or_else(|| McpError::new(
                crate::protocol::StandardErrorCode::PromptNotFound,
                format!("Prompt '{}' not found", name)
            ))?;

        prompt.get_prompt(arguments).await
            .map_err(|e| McpError::new(
                crate::protocol::StandardErrorCode::PromptExecutionError,
                format!("Prompt execution failed: {}", e)
            ).into())
    }

    pub fn len(&self) -> usize {
        self.prompts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.prompts.is_empty()
    }
}

impl Default for PromptRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Built-in prompt providers
pub struct SimplePrompt {
    name: String,
    description: Option<String>,
    template: String,
    arguments: Vec<PromptArgument>,
}

impl SimplePrompt {
    pub fn new(name: impl Into<String>, template: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            template: template.into(),
            arguments: Vec::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_argument(mut self, name: impl Into<String>, description: Option<String>, required: bool) -> Self {
        self.arguments.push(PromptArgument {
            name: name.into(),
            description,
            required: Some(required),
        });
        self
    }

    fn render_template(&self, arguments: &HashMap<String, String>) -> String {
        let mut rendered = self.template.clone();

        for (key, value) in arguments {
            let placeholder = format!("{{{}}}", key);
            rendered = rendered.replace(&placeholder, value);
        }

        rendered
    }
}

#[async_trait]
impl PromptProvider for SimplePrompt {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn arguments(&self) -> Vec<PromptArgument> {
        self.arguments.clone()
    }

    async fn get_prompt(&self, arguments: HashMap<String, String>) -> Result<GetPromptResult> {
        // Validate required arguments
        for arg in &self.arguments {
            if arg.required.unwrap_or(false) && !arguments.contains_key(&arg.name) {
                return Err(McpError::invalid_params(
                    format!("Missing required argument: {}", arg.name)
                ).into());
            }
        }

        let content = self.render_template(&arguments);

        Ok(GetPromptResult {
            description: self.description.clone(),
            messages: vec![PromptMessage {
                role: crate::protocol::PromptRole::User,
                content: crate::protocol::Content::text(content),
            }],
        })
    }
}

pub struct CodeReviewPrompt;

impl CodeReviewPrompt {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl PromptProvider for CodeReviewPrompt {
    fn name(&self) -> &str {
        "code_review"
    }

    fn description(&self) -> Option<&str> {
        Some("Generate a code review prompt for the given code")
    }

    fn arguments(&self) -> Vec<PromptArgument> {
        vec![
            PromptArgument {
                name: "code".to_string(),
                description: Some("The code to review".to_string()),
                required: Some(true),
            },
            PromptArgument {
                name: "language".to_string(),
                description: Some("Programming language of the code".to_string()),
                required: Some(false),
            },
            PromptArgument {
                name: "focus".to_string(),
                description: Some("Specific areas to focus on (security, performance, style, etc.)".to_string()),
                required: Some(false),
            },
        ]
    }

    async fn get_prompt(&self, arguments: HashMap<String, String>) -> Result<GetPromptResult> {
        let code = arguments.get("code")
            .ok_or_else(|| McpError::invalid_params("Missing required argument: code"))?;

        let language = arguments.get("language").cloned()
            .unwrap_or_else(|| "unknown".to_string());

        let focus = arguments.get("focus").cloned()
            .unwrap_or_else(|| "general code quality, security, and best practices".to_string());

        let prompt = format!(
            r#"Please review the following {} code with a focus on {}:

```{}
{}
```

Provide feedback on:
1. Code quality and readability
2. Potential bugs or issues
3. Performance considerations
4. Security concerns (if applicable)
5. Best practices and style
6. Suggestions for improvement

Please be constructive and specific in your feedback."#,
            language, focus, language, code
        );

        Ok(GetPromptResult {
            description: Some(format!("Code review for {} code", language)),
            messages: vec![PromptMessage {
                role: crate::protocol::PromptRole::User,
                content: crate::protocol::Content::text(prompt),
            }],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_prompt_registry() {
        let mut registry = PromptRegistry::new();

        let prompt = SimplePrompt::new("greeting", "Hello, {name}!")
            .with_description("A simple greeting prompt")
            .with_argument("name", Some("The name to greet".to_string()), true);

        registry.register(Box::new(prompt)).await.unwrap();

        // Test listing prompts
        let prompts = registry.list_prompts().await.unwrap();
        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].name, "greeting");

        // Test getting prompt
        let mut args = HashMap::new();
        args.insert("name".to_string(), "World".to_string());

        let result = registry.get_prompt("greeting", args).await.unwrap();
        assert_eq!(result.messages.len(), 1);

        if let crate::protocol::Content::Text { text } = &result.messages[0].content {
            assert_eq!(text, "Hello, World!");
        } else {
            panic!("Expected text content");
        }
    }

    #[tokio::test]
    async fn test_code_review_prompt() {
        let prompt = CodeReviewPrompt::new();

        let mut args = HashMap::new();
        args.insert("code".to_string(), "fn main() { println!(\"Hello\"); }".to_string());
        args.insert("language".to_string(), "rust".to_string());

        let result = prompt.get_prompt(args).await.unwrap();
        assert_eq!(result.messages.len(), 1);

        if let crate::protocol::Content::Text { text } = &result.messages[0].content {
            assert!(text.contains("rust"));
            assert!(text.contains("fn main()"));
        } else {
            panic!("Expected text content");
        }
    }
}