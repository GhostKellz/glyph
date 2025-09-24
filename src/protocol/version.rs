use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProtocolVersion(pub String);

impl ProtocolVersion {
    pub const V_2024_11_05: Self = Self::new("2024-11-05");
    pub const V_2025_03_26: Self = Self::new("2025-03-26");
    pub const V_2025_06_18: Self = Self::new("2025-06-18");
    pub const LATEST: Self = Self::V_2025_03_26;

    const fn new(_version: &'static str) -> Self {
        Self(String::new()) // Placeholder for const context
    }

    pub fn from_str(s: &str) -> Self {
        Self(s.to_string())
    }

    pub fn is_supported(&self) -> bool {
        matches!(self.0.as_str(), "2024-11-05" | "2025-03-26" | "2025-06-18")
    }

    pub fn negotiate(client: &Self, server: &Self) -> Option<Self> {
        if client == server {
            return Some(client.clone());
        }

        // Try to find a common supported version
        let versions = ["2025-06-18", "2025-03-26", "2024-11-05"];
        for v in versions {
            let version = Self::from_str(v);
            if client == &version || server == &version {
                return Some(version);
            }
        }

        None
    }
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self::LATEST
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}