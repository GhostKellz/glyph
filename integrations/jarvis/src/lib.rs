/// Jarvis CLI Integration for Glyph MCP
///
/// Wraps Glyph server as optional backend with:
/// - Interactive consent prompts for sensitive operations
/// - Tool scope management
/// - Policy configuration
/// - Audit logging

use glyph::server::{Server, Tool, ToolContext};
use glyph::protocol::{CallToolResult, Content, ToolInputSchema};
use glyph::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
// Policy Configuration
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Global consent mode
    pub consent_mode: ConsentMode,
    /// Tool-specific policies
    pub tool_policies: HashMap<String, ToolPolicy>,
    /// Scope permissions
    pub scopes: HashMap<String, ScopeConfig>,
    /// Audit settings
    pub audit: AuditConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConsentMode {
    /// Always ask for consent
    Always,
    /// Ask once per session
    Once,
    /// Never ask (auto-approve)
    Never,
    /// Use tool-specific policies
    PerTool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolPolicy {
    pub consent_required: bool,
    pub scopes: Vec<String>,
    pub rate_limit: Option<RateLimit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub max_calls: u32,
    pub per_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeConfig {
    pub name: String,
    pub description: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub enabled: bool,
    pub log_file: Option<PathBuf>,
    pub include_args: bool,
    pub include_results: bool,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        let mut scopes = HashMap::new();
        scopes.insert(
            "fs.read".to_string(),
            ScopeConfig {
                name: "fs.read".to_string(),
                description: "Read files from filesystem".to_string(),
                permissions: vec!["read".to_string()],
            },
        );
        scopes.insert(
            "fs.write".to_string(),
            ScopeConfig {
                name: "fs.write".to_string(),
                description: "Write files to filesystem".to_string(),
                permissions: vec!["write".to_string()],
            },
        );
        scopes.insert(
            "shell.execute".to_string(),
            ScopeConfig {
                name: "shell.execute".to_string(),
                description: "Execute shell commands".to_string(),
                permissions: vec!["execute".to_string()],
            },
        );

        Self {
            consent_mode: ConsentMode::PerTool,
            tool_policies: HashMap::new(),
            scopes,
            audit: AuditConfig {
                enabled: true,
                log_file: None,
                include_args: true,
                include_results: false,
            },
        }
    }
}

// ============================================================================
// Consent Guard
// ============================================================================

pub struct ConsentGuard {
    policy: Arc<RwLock<PolicyConfig>>,
    approved_tools: Arc<RwLock<HashSet<String>>>,
}

impl ConsentGuard {
    pub fn new(policy: PolicyConfig) -> Self {
        Self {
            policy: Arc::new(RwLock::new(policy)),
            approved_tools: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub async fn require(&self, tool_name: &str, scope: &str) -> std::result::Result<(), String> {
        let policy = self.policy.read().await;

        // Check if tool has been approved this session
        let approved = self.approved_tools.read().await;
        if approved.contains(tool_name) {
            return Ok(());
        }

        // Check consent mode
        match &policy.consent_mode {
            ConsentMode::Never => return Ok(()),
            ConsentMode::Always => {
                // Prompt user (in real implementation, would use dialoguer)
                return self.prompt_user(tool_name, scope).await;
            }
            ConsentMode::Once => {
                let result = self.prompt_user(tool_name, scope).await;
                if result.is_ok() {
                    let mut approved = self.approved_tools.write().await;
                    approved.insert(tool_name.to_string());
                }
                return result;
            }
            ConsentMode::PerTool => {
                if let Some(tool_policy) = policy.tool_policies.get(tool_name) {
                    if tool_policy.consent_required {
                        return self.prompt_user(tool_name, scope).await;
                    }
                }
                return Ok(());
            }
        }
    }

    async fn prompt_user(&self, tool_name: &str, scope: &str) -> std::result::Result<(), String> {
        // In real implementation, use dialoguer for interactive prompts
        // For now, just approve
        tracing::warn!("Consent required for {} (scope: {})", tool_name, scope);
        Ok(())
    }
}

// ============================================================================
// Jarvis Tool Wrapper
// ============================================================================

/// Wraps any Glyph tool with Jarvis consent and auditing
pub struct JarvisTool<T: Tool> {
    inner: T,
    guard: Arc<ConsentGuard>,
    audit_logger: Arc<AuditLogger>,
}

impl<T: Tool> JarvisTool<T> {
    pub fn new(tool: T, guard: Arc<ConsentGuard>, audit_logger: Arc<AuditLogger>) -> Self {
        Self {
            inner: tool,
            guard,
            audit_logger,
        }
    }
}

#[async_trait]
impl<T: Tool + Send + Sync> Tool for JarvisTool<T> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> Option<&str> {
        self.inner.description()
    }

    fn input_schema(&self) -> ToolInputSchema {
        self.inner.input_schema()
    }

    async fn call(&self, args: Option<Value>) -> Result<CallToolResult> {
        let tool_name = self.name();

        // Check consent
        let scope = self.determine_scope(tool_name);
        if let Err(e) = self.guard.require(tool_name, &scope).await {
            return Ok(CallToolResult {
                content: vec![Content::text(format!("Permission denied: {}", e))],
                is_error: Some(true),
                meta: Some(serde_json::json!({
                    "reason": "consent_denied",
                    "scope": scope,
                })),
            });
        }

        // Audit log (before execution)
        self.audit_logger
            .log_call(tool_name, args.as_ref())
            .await;

        // Execute tool
        let result = self.inner.call(args).await;

        // Audit log (after execution)
        if let Ok(ref call_result) = result {
            self.audit_logger
                .log_result(tool_name, call_result)
                .await;
        }

        result
    }
}

impl<T: Tool> JarvisTool<T> {
    fn determine_scope(&self, tool_name: &str) -> String {
        match tool_name {
            "read_file" | "list_directory" => "fs.read".to_string(),
            "write_file" | "delete_file" => "fs.write".to_string(),
            "shell_execute" => "shell.execute".to_string(),
            _ => "default".to_string(),
        }
    }
}

// ============================================================================
// Audit Logger
// ============================================================================

pub struct AuditLogger {
    config: Arc<RwLock<AuditConfig>>,
}

impl AuditLogger {
    pub fn new(config: AuditConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    pub async fn log_call(&self, tool_name: &str, args: Option<&Value>) {
        let config = self.config.read().await;
        if !config.enabled {
            return;
        }

        let timestamp = chrono::Utc::now();
        let mut log_entry = serde_json::json!({
            "timestamp": timestamp.to_rfc3339(),
            "event": "tool_call",
            "tool": tool_name,
        });

        if config.include_args {
            log_entry["args"] = args.cloned().unwrap_or(Value::Null);
        }

        tracing::info!(target: "audit", "{}", serde_json::to_string(&log_entry).unwrap());

        // Write to log file if configured
        if let Some(log_file) = &config.log_file {
            if let Err(e) = self.write_to_file(log_file, &log_entry).await {
                tracing::error!("Failed to write audit log: {}", e);
            }
        }
    }

    pub async fn log_result(&self, tool_name: &str, result: &CallToolResult) {
        let config = self.config.read().await;
        if !config.enabled || !config.include_results {
            return;
        }

        let timestamp = chrono::Utc::now();
        let log_entry = serde_json::json!({
            "timestamp": timestamp.to_rfc3339(),
            "event": "tool_result",
            "tool": tool_name,
            "is_error": result.is_error,
        });

        tracing::info!(target: "audit", "{}", serde_json::to_string(&log_entry).unwrap());
    }

    async fn write_to_file(&self, path: &PathBuf, entry: &Value) -> std::io::Result<()> {
        use tokio::io::AsyncWriteExt;

        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await?;

        file.write_all(entry.to_string().as_bytes()).await?;
        file.write_all(b"\n").await?;
        file.flush().await?;

        Ok(())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Load policy from config file
pub fn load_policy(config_path: &PathBuf) -> anyhow::Result<PolicyConfig> {
    let content = std::fs::read_to_string(config_path)?;
    let policy: PolicyConfig = toml::from_str(&content)?;
    Ok(policy)
}

/// Save policy to config file
pub fn save_policy(policy: &PolicyConfig, config_path: &PathBuf) -> anyhow::Result<()> {
    let content = toml::to_string_pretty(policy)?;
    std::fs::write(config_path, content)?;
    Ok(())
}

/// Get default config path
pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("jarvis")
        .join("policy.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policy() {
        let policy = PolicyConfig::default();
        assert!(policy.scopes.contains_key("fs.read"));
        assert!(policy.scopes.contains_key("fs.write"));
        assert!(policy.scopes.contains_key("shell.execute"));
    }

    #[tokio::test]
    async fn test_consent_guard() {
        let policy = PolicyConfig::default();
        let guard = ConsentGuard::new(policy);

        // Should approve (for test)
        let result = guard.require("test_tool", "default").await;
        assert!(result.is_ok());
    }
}
