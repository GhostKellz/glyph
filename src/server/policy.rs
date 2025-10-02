/// Policy Engine for Glyph MCP
///
/// Implements consent gates, audit trails, and permission management.

use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, warn};

// ============================================================================
// Policy Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub rules: Vec<PolicyRule>,
    pub audit: AuditPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRule {
    pub name: String,
    pub description: Option<String>,
    pub condition: PolicyCondition,
    pub action: PolicyAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PolicyCondition {
    ToolName { matches: String },
    Scope { required: Vec<String> },
    RateLimit { max_per_second: u32 },
    Always,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PolicyAction {
    Allow,
    Deny { reason: String },
    RequireConsent { message: String },
    Audit { level: AuditLevel },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuditLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditPolicy {
    pub enabled: bool,
    pub log_requests: bool,
    pub log_responses: bool,
    pub redact_secrets: bool,
}

impl Default for AuditPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            log_requests: true,
            log_responses: false,
            redact_secrets: true,
        }
    }
}

// ============================================================================
// Policy Engine
// ============================================================================

pub struct PolicyEngine {
    policy: Arc<RwLock<Policy>>,
    approved_operations: Arc<RwLock<HashSet<String>>>,
    audit_trail: Arc<RwLock<Vec<AuditEntry>>>,
}

impl PolicyEngine {
    pub fn new(policy: Policy) -> Self {
        Self {
            policy: Arc::new(RwLock::new(policy)),
            approved_operations: Arc::new(RwLock::new(HashSet::new())),
            audit_trail: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub async fn check_permission(
        &self,
        tool_name: &str,
        scope: &str,
    ) -> std::result::Result<(), String> {
        let policy = self.policy.read().await;

        for rule in &policy.rules {
            if self.matches_condition(&rule.condition, tool_name, scope) {
                match &rule.action {
                    PolicyAction::Allow => {
                        debug!("Policy allows: {} ({})", tool_name, rule.name);
                        return Ok(());
                    }
                    PolicyAction::Deny { reason } => {
                        warn!("Policy denies: {} - {}", tool_name, reason);
                        return Err(reason.clone());
                    }
                    PolicyAction::RequireConsent { message } => {
                        return self.handle_consent(tool_name, message).await;
                    }
                    PolicyAction::Audit { level } => {
                        self.audit_operation(tool_name, scope, *level).await;
                    }
                }
            }
        }

        // Default: allow
        Ok(())
    }

    fn matches_condition(
        &self,
        condition: &PolicyCondition,
        tool_name: &str,
        _scope: &str,
    ) -> bool {
        match condition {
            PolicyCondition::ToolName { matches } => tool_name == matches,
            PolicyCondition::Scope { required: _ } => true, // Simplified
            PolicyCondition::RateLimit { max_per_second: _ } => true, // Simplified
            PolicyCondition::Always => true,
        }
    }

    async fn handle_consent(&self, tool_name: &str, message: &str) -> std::result::Result<(), String> {
        // Check if already approved
        let approved = self.approved_operations.read().await;
        if approved.contains(tool_name) {
            return Ok(());
        }
        drop(approved);

        // In production, this would prompt the user
        warn!("Consent required for {}: {}", tool_name, message);

        // For now, auto-approve
        let mut approved = self.approved_operations.write().await;
        approved.insert(tool_name.to_string());

        Ok(())
    }

    async fn audit_operation(&self, tool_name: &str, scope: &str, level: AuditLevel) {
        let entry = AuditEntry {
            timestamp: chrono::Utc::now(),
            tool: tool_name.to_string(),
            scope: scope.to_string(),
            level,
            approved: true,
        };

        let mut trail = self.audit_trail.write().await;
        trail.push(entry.clone());

        // Log to tracing
        match level {
            AuditLevel::Debug => debug!(target: "audit", "{:?}", entry),
            AuditLevel::Info => tracing::info!(target: "audit", "{:?}", entry),
            AuditLevel::Warn => warn!(target: "audit", "{:?}", entry),
            AuditLevel::Error => tracing::error!(target: "audit", "{:?}", entry),
        }
    }

    pub async fn get_audit_trail(&self) -> Vec<AuditEntry> {
        self.audit_trail.read().await.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub tool: String,
    pub scope: String,
    pub level: AuditLevel,
    pub approved: bool,
}

// ============================================================================
// Default Policies
// ============================================================================

impl Default for Policy {
    fn default() -> Self {
        Self {
            rules: vec![
                PolicyRule {
                    name: "audit_shell_commands".to_string(),
                    description: Some("Audit all shell command executions".to_string()),
                    condition: PolicyCondition::ToolName {
                        matches: "shell_execute".to_string(),
                    },
                    action: PolicyAction::Audit {
                        level: AuditLevel::Warn,
                    },
                },
                PolicyRule {
                    name: "consent_file_deletion".to_string(),
                    description: Some("Require consent for file deletion".to_string()),
                    condition: PolicyCondition::ToolName {
                        matches: "delete_file".to_string(),
                    },
                    action: PolicyAction::RequireConsent {
                        message: "This will permanently delete a file".to_string(),
                    },
                },
            ],
            audit: AuditPolicy::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_policy_engine() {
        let policy = Policy::default();
        let engine = PolicyEngine::new(policy);

        // Should allow with audit
        let result = engine.check_permission("shell_execute", "shell").await;
        assert!(result.is_ok());

        // Check audit trail
        let trail = engine.get_audit_trail().await;
        assert_eq!(trail.len(), 1);
    }
}
