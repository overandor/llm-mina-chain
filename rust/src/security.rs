//! Security features: input validation, rate limiting, and audit logging

use prometheus::{Histogram, IntCounter, Registry};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

use crate::{Transaction, Block};

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Maximum transaction amount
    pub max_transaction_amount: u64,
    /// Maximum gas limit
    pub max_gas_limit: u64,
    /// Maximum gas price
    pub max_gas_price: u64,
    /// Rate limit requests per minute
    pub rate_limit_rpm: u64,
    /// Maximum block size in bytes
    pub max_block_size: usize,
    /// Maximum transactions per block
    pub max_transactions_per_block: usize,
    /// Enable signature verification
    pub require_signatures: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        SecurityConfig {
            max_transaction_amount: 1_000_000_000,
            max_gas_limit: 10_000_000,
            max_gas_price: 1_000,
            rate_limit_rpm: 100,
            max_block_size: 10 * 1024 * 1024, // 10MB
            max_transactions_per_block: 10_000,
            require_signatures: true,
        }
    }
}

/// Input validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

impl ValidationResult {
    pub fn new() -> Self {
        ValidationResult {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }
    
    pub fn add_error(&mut self, error: String) {
        self.valid = false;
        self.errors.push(error);
    }
    
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }
    
    pub fn is_valid(&self) -> bool {
        self.valid
    }
}

/// Input validator
pub struct InputValidator {
    config: SecurityConfig,
    metrics: SecurityMetrics,
}

impl InputValidator {
    pub fn new(config: SecurityConfig, registry: &Registry) -> Self {
        let metrics = SecurityMetrics::new(registry);
        InputValidator { config, metrics }
    }
    
    /// Validate a transaction
    pub fn validate_transaction(&self, tx: &Transaction) -> ValidationResult {
        let mut result = ValidationResult::new();
        
        // Check amount
        if tx.amount == 0 {
            result.add_error("Transaction amount cannot be zero".to_string());
        }
        
        if tx.amount > self.config.max_transaction_amount {
            result.add_error(format!(
                "Transaction amount {} exceeds maximum {}",
                tx.amount, self.config.max_transaction_amount
            ));
        }
        
        // Check gas parameters
        if let Some(gas_limit) = tx.gas_limit {
            if gas_limit > self.config.max_gas_limit {
                result.add_error(format!(
                    "Gas limit {} exceeds maximum {}",
                    gas_limit, self.config.max_gas_limit
                ));
            }
            
            if gas_limit < 21000 {
                result.add_error("Gas limit too low (minimum 21000)".to_string());
            }
        }
        
        if let Some(gas_price) = tx.gas_price {
            if gas_price > self.config.max_gas_price {
                result.add_error(format!(
                    "Gas price {} exceeds maximum {}",
                    gas_price, self.config.max_gas_price
                ));
            }
        }
        
        // Check sender and receiver are different
        if tx.sender == tx.receiver {
            result.add_error("Sender and receiver cannot be the same".to_string());
        }
        
        // Check addresses are valid (basic check)
        if !self.is_valid_address(&tx.sender) {
            result.add_error("Invalid sender address".to_string());
        }
        
        if !self.is_valid_address(&tx.receiver) {
            result.add_error("Invalid receiver address".to_string());
        }
        
        // Check signature if required, and verify cryptographically when present
        if self.config.require_signatures {
            if tx.signature.is_none() {
                result.add_error("Signature required but not provided".to_string());
            } else if let Ok(public_key) = crate::PublicKey::from_hex(&tx.sender) {
                if let Err(_e) = tx.verify_signature(&public_key) {
                    result.add_error("Invalid transaction signature".to_string());
                }
            } else {
                result.add_error("Invalid sender public key format".to_string());
            }
        }
        
        // Update metrics
        if result.is_valid() {
            self.metrics.valid_transactions.inc();
        } else {
            self.metrics.invalid_transactions.inc();
        }
        
        result
    }
    
    /// Validate a block
    pub fn validate_block(&self, block: &Block) -> ValidationResult {
        let mut result = ValidationResult::new();
        
        // Check block size
        let block_size = serde_json::to_vec(block).unwrap_or_default().len();
        if block_size > self.config.max_block_size {
            result.add_error(format!(
                "Block size {} bytes exceeds maximum {}",
                block_size, self.config.max_block_size
            ));
        }
        
        // Check transaction count
        if block.transactions.len() > self.config.max_transactions_per_block {
            result.add_error(format!(
                "Transaction count {} exceeds maximum {}",
                block.transactions.len(),
                self.config.max_transactions_per_block
            ));
        }
        
        // Validate all transactions
        for (i, tx) in block.transactions.iter().enumerate() {
            let tx_result = self.validate_transaction(tx);
            if !tx_result.is_valid() {
                result.add_error(format!(
                    "Transaction {} invalid: {}",
                    i,
                    tx_result.errors.join(", ")
                ));
            }
        }
        
        // Update metrics
        if result.is_valid() {
            self.metrics.valid_blocks.inc();
        } else {
            self.metrics.invalid_blocks.inc();
        }
        
        result
    }
    
    /// Validate an address (basic check)
    fn is_valid_address(&self, address: &str) -> bool {
        // Basic validation: non-empty, reasonable length, alphanumeric
        !address.is_empty() && address.len() <= 64 && address.chars().all(|c| c.is_alphanumeric())
    }
}

/// Rate limiter using token bucket algorithm
#[derive(Clone)]
pub struct RateLimiter {
    capacity: u64,
    tokens: Arc<RwLock<u64>>,
    last_refill: Arc<RwLock<Instant>>,
    refill_rate: u64,
}

impl RateLimiter {
    pub fn new(capacity: u64, refill_rate: u64) -> Self {
        RateLimiter {
            capacity,
            tokens: Arc::new(RwLock::new(capacity)),
            last_refill: Arc::new(RwLock::new(Instant::now())),
            refill_rate,
        }
    }
    
    /// Check if a request is allowed
    pub async fn check(&self) -> bool {
        let mut tokens = self.tokens.write().await;
        let mut last_refill = self.last_refill.write().await;
        
        // Refill tokens
        let now = Instant::now();
        let elapsed = now.duration_since(*last_refill);
        let tokens_to_add = (elapsed.as_secs() as u64) * self.refill_rate;
        
        *tokens = (*tokens + tokens_to_add).min(self.capacity);
        *last_refill = now;
        
        // Check if we have tokens
        if *tokens > 0 {
            *tokens -= 1;
            true
        } else {
            false
        }
    }
    
    /// Get current token count
    pub async fn token_count(&self) -> u64 {
        let tokens = self.tokens.read().await;
        *tokens
    }
}

/// Per-IP rate limiter
pub struct IpRateLimiter {
    limiters: Arc<RwLock<HashMap<IpAddr, RateLimiter>>>,
    config: SecurityConfig,
}

impl IpRateLimiter {
    pub fn new(config: SecurityConfig) -> Self {
        IpRateLimiter {
            limiters: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }
    
    /// Check if an IP is rate limited
    pub async fn check(&self, ip: IpAddr) -> bool {
        // First, get or create the limiter
        let limiter = {
            let mut limiters = self.limiters.write().await;
            limiters
                .entry(ip)
                .or_insert_with(|| RateLimiter::new(self.config.rate_limit_rpm, self.config.rate_limit_rpm / 60))
                .clone()
        };
        // Then check it without holding the lock
        limiter.check().await
    }
    
    /// Clean up old entries (remove limiters with zero tokens)
    pub async fn cleanup(&self) {
        let mut limiters = self.limiters.write().await;
        let mut to_remove = Vec::new();
        for (ip, limiter) in limiters.iter() {
            let count = limiter.token_count().await;
            if count == 0 {
                to_remove.push(*ip);
            }
        }
        for ip in to_remove {
            limiters.remove(&ip);
        }
    }
}

/// Security metrics
pub struct SecurityMetrics {
    pub valid_transactions: IntCounter,
    pub invalid_transactions: IntCounter,
    pub valid_blocks: IntCounter,
    pub invalid_blocks: IntCounter,
    pub rate_limit_exceeded: IntCounter,
    pub validation_duration: Histogram,
}

impl SecurityMetrics {
    pub fn new(_registry: &Registry) -> Self {
        SecurityMetrics {
            valid_transactions: IntCounter::new(
                "security_valid_transactions_total",
                "Total number of valid transactions",
            ).unwrap(),
            invalid_transactions: IntCounter::new(
                "security_invalid_transactions_total",
                "Total number of invalid transactions",
            ).unwrap(),
            valid_blocks: IntCounter::new(
                "security_valid_blocks_total",
                "Total number of valid blocks",
            ).unwrap(),
            invalid_blocks: IntCounter::new(
                "security_invalid_blocks_total",
                "Total number of invalid blocks",
            ).unwrap(),
            rate_limit_exceeded: IntCounter::new(
                "security_rate_limit_exceeded_total",
                "Total number of rate limit violations",
            ).unwrap(),
            validation_duration: Histogram::with_opts(
                prometheus::HistogramOpts::new(
                    "security_validation_duration_seconds",
                    "Time spent validating inputs",
                )
                .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]),
            )
            .unwrap(),
        }
    }
}

/// Audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub timestamp: i64,
    pub event_type: String,
    pub source: String,
    pub details: serde_json::Value,
}

/// Audit logger
pub struct AuditLogger {
    entries: Arc<RwLock<Vec<AuditLogEntry>>>,
    max_entries: usize,
}

impl AuditLogger {
    pub fn new(max_entries: usize) -> Self {
        AuditLogger {
            entries: Arc::new(RwLock::new(Vec::new())),
            max_entries,
        }
    }
    
    /// Log an event
    pub async fn log(&self, event_type: String, source: String, details: serde_json::Value) {
        let mut entries = self.entries.write().await;
        
        let entry = AuditLogEntry {
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
            event_type,
            source,
            details,
        };
        
        entries.push(entry);
        
        // Trim if too many entries
        if entries.len() > self.max_entries {
            entries.remove(0);
        }
    }
    
    /// Get recent entries
    pub async fn get_recent(&self, limit: usize) -> Vec<AuditLogEntry> {
        let entries = self.entries.read().await;
        let start = if entries.len() > limit {
            entries.len() - limit
        } else {
            0
        };
        entries[start..].to_vec()
    }
}
