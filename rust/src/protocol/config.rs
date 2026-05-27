use serde::{Deserialize, Serialize};
use std::path::Path;

use super::types::SemVer;

/// One canonical config structure loaded by ALL agents.
/// Each agent reads only the sections it cares about.
/// No agent may define its own separate config type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalConfig {
    /// Protocol version this config was written for.
    pub protocol_version: SemVer,
    /// Core runtime configuration.
    pub core: CoreConfig,
    /// Storage configuration.
    pub storage: StorageConfig,
    /// Solana query configuration.
    pub solana: SolanaConfig,
    /// Proof / provenance configuration.
    pub proof: ProofConfig,
    /// API server configuration.
    pub api: ApiConfig,
    /// Network configuration.
    pub network: NetworkConfig,
    /// Logging configuration.
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    pub chain_id: String,
    pub block_time_ms: u64,
    pub max_block_size: usize,
    pub gas_enabled: bool,
    pub deterministic_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub db_path: String,
    pub cache_size_mb: usize,
    pub flush_interval_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaConfig {
    pub rpc_endpoint: String,
    pub commitment: String,
    pub max_concurrent_queries: usize,
    pub query_timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofConfig {
    pub merkle_tree_depth: u8,
    pub anchor_interval_blocks: u64,
    pub ipfs_gateway: String,
    pub solana_program_id: String,
    pub receipt_ttl_days: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub bind_addr: String,
    pub request_timeout_ms: u64,
    pub rate_limit_per_min: u64,
    pub api_version_header: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub listen_addrs: Vec<String>,
    pub bootstrap_nodes: Vec<String>,
    pub max_peers: usize,
    pub enable_mdns: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub output_path: Option<String>,
    pub structured: bool,
}

impl CanonicalConfig {
    /// Load from a TOML file. Fails if the file does not exist or is invalid.
    /// No silent fallback to defaults.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let contents = std::fs::read_to_string(path)?;
        let config: CanonicalConfig = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Load from environment variables prefixed with `LLM_MINA_`.
    /// Falls back to sensible defaults ONLY for fields not present.
    pub fn from_env_or_default() -> Self {
        // We intentionally do not use silent fallback for critical fields.
        // Build a base config from defaults, then overlay env vars.
        let mut builder = config::Config::builder();

        // Set defaults
        builder = builder.set_default("protocol_version.major", 0u16).unwrap();
        builder = builder.set_default("protocol_version.minor", 1u16).unwrap();
        builder = builder.set_default("protocol_version.patch", 0u16).unwrap();
        builder = builder.set_default("core.chain_id", "llm-mina-dev").unwrap();
        builder = builder.set_default("core.block_time_ms", 2000u64).unwrap();
        builder = builder.set_default("core.max_block_size", 1000i64).unwrap();
        builder = builder.set_default("core.gas_enabled", true).unwrap();
        builder = builder.set_default("core.deterministic_mode", true).unwrap();

        builder = builder.set_default("storage.db_path", "./data").unwrap();
        builder = builder.set_default("storage.cache_size_mb", 64i64).unwrap();
        builder = builder.set_default("storage.flush_interval_sec", 30i64).unwrap();

        builder = builder
            .set_default("solana.rpc_endpoint", "https://api.mainnet-beta.solana.com")
            .unwrap();
        builder = builder.set_default("solana.commitment", "confirmed").unwrap();
        builder = builder.set_default("solana.max_concurrent_queries", 10i64).unwrap();
        builder = builder.set_default("solana.query_timeout_ms", 30000u64).unwrap();

        builder = builder.set_default("proof.merkle_tree_depth", 20u8).unwrap();
        builder = builder.set_default("proof.anchor_interval_blocks", 100u64).unwrap();
        builder = builder
            .set_default("proof.ipfs_gateway", "https://ipfs.io")
            .unwrap();
        builder = builder
            .set_default("proof.solana_program_id", "")
            .unwrap();
        builder = builder.set_default("proof.receipt_ttl_days", 365u64).unwrap();

        builder = builder
            .set_default("api.bind_addr", "0.0.0.0:8000")
            .unwrap();
        builder = builder.set_default("api.request_timeout_ms", 30000u64).unwrap();
        builder = builder.set_default("api.rate_limit_per_min", 100u64).unwrap();
        builder = builder.set_default("api.api_version_header", "X-API-Version").unwrap();

        builder = builder.set_default("network.listen_addrs", vec!["/ip4/0.0.0.0/tcp/0"]).unwrap();
        builder = builder.set_default("network.bootstrap_nodes", Vec::<String>::new()).unwrap();
        builder = builder.set_default("network.max_peers", 50u64).unwrap();
        builder = builder.set_default("network.enable_mdns", true).unwrap();

        builder = builder.set_default("logging.level", "info").unwrap();
        builder = builder.set_default("logging.format", "json").unwrap();
        builder = builder.set_default("logging.structured", true).unwrap();

        // Overlay environment variables
        builder = builder.add_source(
            config::Environment::with_prefix("LLM_MINA")
                .separator("_")
                .try_parsing(true),
        );

        let cfg = builder.build().expect("config build must not fail");
        cfg.try_deserialize().expect("config deserialization must not fail")
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),
}
