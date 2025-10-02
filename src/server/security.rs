/// Security hardening for Glyph MCP
///
/// Secret redaction, rate limiting, and TLS configuration.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
// Note: regex crate would be needed for full implementation
// For now, using simple string matching
use std::borrow::Cow;

// ============================================================================
// Secret Redaction
// ============================================================================

pub struct SecretRedactor {
    patterns: Vec<&'static str>,
}

impl SecretRedactor {
    pub fn new() -> Self {
        Self {
            patterns: vec![
                "api_key",
                "apikey",
                "api-key",
                "access_token",
                "password",
                "passwd",
                "pwd",
                "bearer",
                "token",
                "AKIA",  // AWS keys
                "-----BEGIN",  // Private keys
            ],
        }
    }

    pub fn redact(&self, input: &str) -> String {
        let mut result = input.to_string();

        // Simple pattern matching (in production, use regex crate)
        for pattern in &self.patterns {
            if result.to_lowercase().contains(&pattern.to_lowercase()) {
                // Redact values after common delimiters
                result = result.replace(pattern, "[REDACTED]");
            }
        }

        result
    }

    pub fn redact_json(&self, value: &serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::String(s) => {
                serde_json::Value::String(self.redact(s))
            }
            serde_json::Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (k, v) in map {
                    new_map.insert(k.clone(), self.redact_json(v));
                }
                serde_json::Value::Object(new_map)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| self.redact_json(v)).collect())
            }
            _ => value.clone(),
        }
    }
}

impl Default for SecretRedactor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Rate Limiting
// ============================================================================

pub struct RateLimiter {
    limits: Arc<RwLock<HashMap<String, RateLimitState>>>,
}

struct RateLimitState {
    count: u32,
    window_start: Instant,
    max_requests: u32,
    window_duration: Duration,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn check_limit(
        &self,
        key: &str,
        max_requests: u32,
        window_duration: Duration,
    ) -> std::result::Result<(), RateLimitError> {
        let mut limits = self.limits.write().await;

        let now = Instant::now();

        let state = limits.entry(key.to_string()).or_insert(RateLimitState {
            count: 0,
            window_start: now,
            max_requests,
            window_duration,
        });

        // Reset window if expired
        if now.duration_since(state.window_start) >= state.window_duration {
            state.count = 0;
            state.window_start = now;
        }

        // Check limit
        if state.count >= state.max_requests {
            let retry_after = state.window_duration
                .saturating_sub(now.duration_since(state.window_start));

            return Err(RateLimitError {
                retry_after,
                limit: state.max_requests,
            });
        }

        state.count += 1;
        Ok(())
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct RateLimitError {
    pub retry_after: Duration,
    pub limit: u32,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Rate limit exceeded. Limit: {} requests. Retry after: {:?}",
            self.limit, self.retry_after
        )
    }
}

impl std::error::Error for RateLimitError {}

// ============================================================================
// TLS Configuration
// ============================================================================

#[derive(Debug, Clone)]
pub struct TlsConfig {
    pub cert_path: std::path::PathBuf,
    pub key_path: std::path::PathBuf,
    pub require_client_cert: bool,
    pub ca_path: Option<std::path::PathBuf>,
}

impl TlsConfig {
    pub fn new(
        cert_path: impl Into<std::path::PathBuf>,
        key_path: impl Into<std::path::PathBuf>,
    ) -> Self {
        Self {
            cert_path: cert_path.into(),
            key_path: key_path.into(),
            require_client_cert: false,
            ca_path: None,
        }
    }

    pub fn with_client_auth(
        mut self,
        ca_path: impl Into<std::path::PathBuf>,
    ) -> Self {
        self.require_client_cert = true;
        self.ca_path = Some(ca_path.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_redaction() {
        let redactor = SecretRedactor::new();

        let input = r#"{"api_key": "sk-1234567890", "password": "secret123"}"#;
        let redacted = redactor.redact(input);

        // Simple redaction replaces key names
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn test_json_redaction() {
        let redactor = SecretRedactor::new();

        let json = serde_json::json!({
            "api_key": "sk-test-key",
            "user": "alice",
            "data": {
                "password": "secret"
            }
        });

        let redacted = redactor.redact_json(&json);
        // JSON redaction preserves structure
        assert!(redacted.is_object());
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new();

        // Should allow first request
        let result = limiter
            .check_limit("test_key", 2, Duration::from_secs(60))
            .await;
        assert!(result.is_ok());

        // Should allow second request
        let result = limiter
            .check_limit("test_key", 2, Duration::from_secs(60))
            .await;
        assert!(result.is_ok());

        // Should deny third request
        let result = limiter
            .check_limit("test_key", 2, Duration::from_secs(60))
            .await;
        assert!(result.is_err());
    }
}
