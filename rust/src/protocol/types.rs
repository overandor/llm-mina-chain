use serde::{Deserialize, Serialize};

/// Canonical deterministic 32-byte hash (SHA-256).
pub type DeterministicHash = [u8; 32];

/// Semantic version used across all agents.
/// Every receipt, config file, and API response carries this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SemVer {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl SemVer {
    pub const CURRENT: Self = Self {
        major: 0,
        minor: 1,
        patch: 0,
    };

    pub fn new(major: u16, minor: u16, patch: u16) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

}

impl std::fmt::Display for SemVer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl Default for SemVer {
    fn default() -> Self {
        Self::CURRENT
    }
}

/// Canonical Unix timestamp in milliseconds.
pub type CanonicalTimestamp = u64;

/// Agent identifier. Used in receipts and logs to identify which agent produced output.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentId {
    CoreRuntime,
    SolanaQuery,
    ProofProvenance,
}

impl AgentId {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentId::CoreRuntime => "core_runtime",
            AgentId::SolanaQuery => "solana_query",
            AgentId::ProofProvenance => "proof_provenance",
        }
    }
}

/// API version header value. All HTTP responses must include this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiVersion {
    pub semver: SemVer,
}

impl ApiVersion {
    pub const CURRENT: Self = Self {
        semver: SemVer::CURRENT,
    };
}

impl Default for ApiVersion {
    fn default() -> Self {
        Self::CURRENT
    }
}
