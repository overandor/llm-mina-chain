//! LLM-Mina-Chain: A micro LLM-based blockchain with atomic instant transactions and optional gas
//! Inspired by Mina Protocol's recursive proof system

pub mod llm_layer;
pub mod crypto;
pub mod storage;
#[cfg(feature = "network")]
pub mod network;
pub mod consensus;
pub mod security;
pub mod metrics;
pub mod zkproof;
pub mod api;
pub mod health;
pub mod solana_agent;

#[cfg(target_arch = "wasm32")]
pub mod wasm_mining;

use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;

pub use llm_layer::{LLMTransactionParser, ParsedTransaction, GasSuggestion};
pub use crypto::{KeyPair, PublicKey, PrivateKey, DigitalSignature, CryptoError};
pub use storage::{BlockchainStorage, StorageError};
#[cfg(feature = "network")]
pub use network::{P2PNode, NetworkConfig, NetworkMessage};
pub use consensus::{HotStuffConsensus, ConsensusMessage, ConsensusAction, ConsensusError, Phase};
pub use security::{InputValidator, SecurityConfig, ValidationResult, RateLimiter, IpRateLimiter, AuditLogger};
pub use metrics::{BlockchainMetrics, MetricsServer, Timer};
pub use zkproof::{ProofSystem, ZkProof, StateTransitionCircuit, ProofError, RecursiveProof, ProofCache};
pub use api::{ApiVersion, ApiEndpoint, ApiRegistry, ApiHandler, ApiRequest, ApiResponse, ApiError};
pub use health::{HealthChecker, HealthCheck, HealthStatus, HealthStatusResponse, AlertManager, Alert, AlertSeverity, SystemMetrics};

#[cfg(target_arch = "wasm32")]
pub use wasm_mining::{MiningWorker, BrowserNode, log_message, get_browser_info};

/// Transaction type enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransactionType {
    Transfer,
    ContractCall,
    LLMGenerated,
}

/// Atomic transaction with optional gas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub tx_id: String,
    pub sender: String,
    pub receiver: String,
    pub amount: u64,
    pub nonce: u64,
    pub gas_limit: Option<u64>,
    pub gas_price: Option<u64>,
    pub tx_type: String,
    pub data: Option<serde_json::Value>,
    pub signature: Option<DigitalSignature>,
    pub timestamp: i64,
}

impl Transaction {
    pub fn new(
        sender: String,
        receiver: String,
        amount: u64,
        nonce: u64,
        gas_limit: Option<u64>,
        gas_price: Option<u64>,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        let tx_id = Self::generate_id(&sender, &receiver, amount, nonce, timestamp);
        
        Transaction {
            tx_id,
            sender,
            receiver,
            amount,
            nonce,
            gas_limit,
            gas_price,
            tx_type: "transfer".to_string(),
            data: None,
            signature: None,
            timestamp,
        }
    }
    
    /// Sign the transaction with a key pair
    pub fn sign(&mut self, keypair: &KeyPair) {
        let message = self.signing_hash();
        self.signature = Some(keypair.sign(&message));
    }
    
    /// Verify the transaction signature
    pub fn verify_signature(&self, public_key: &PublicKey) -> Result<(), CryptoError> {
        if let Some(ref signature) = self.signature {
            let message = self.signing_hash();
            public_key.verify(&message, signature)
        } else {
            Err(CryptoError::InvalidSignature)
        }
    }
    
    /// Get the signing hash (all fields except signature)
    fn signing_hash(&self) -> Vec<u8> {
        let data = format!(
            "{}{}{}{}{:?}{:?}{:?}{:?}{:?}",
            self.tx_id, self.sender, self.receiver, self.amount, 
            self.nonce, self.gas_limit, self.gas_price, self.tx_type, self.timestamp
        );
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hasher.finalize().to_vec()
    }
    
    fn generate_id(sender: &str, receiver: &str, amount: u64, nonce: u64, timestamp: i64) -> String {
        let data = format!("{}{}{}{}{}", sender, receiver, amount, nonce, timestamp);
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        let result = hasher.finalize();
        hex::encode(result)[..16].to_string()
    }
    
    pub fn hash(&self) -> String {
        let data = format!(
            "{}{}{}{}{:?}{:?}{:?}{:?}{:?}",
            self.tx_id, self.sender, self.receiver, self.amount, 
            self.nonce, self.gas_limit, self.gas_price, self.tx_type, self.timestamp
        );
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hex::encode(hasher.finalize())
    }
    
    pub fn is_gasless(&self) -> bool {
        self.gas_limit.is_none() || self.gas_price.is_none()
    }
    
    pub fn calculate_gas_cost(&self) -> u64 {
        if self.is_gasless() {
            0
        } else {
            self.gas_limit.unwrap_or(0) * self.gas_price.unwrap_or(0)
        }
    }
}

/// Blockchain state (balances, nonces, contracts)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub balances: HashMap<String, u64>,
    pub nonces: HashMap<String, u64>,
    pub contracts: HashMap<String, serde_json::Value>,
}

impl State {
    pub fn new() -> Self {
        State {
            balances: HashMap::new(),
            nonces: HashMap::new(),
            contracts: HashMap::new(),
        }
    }
    
    pub fn get_balance(&self, address: &str) -> u64 {
        *self.balances.get(address).unwrap_or(&0)
    }
    
    pub fn set_balance(&mut self, address: String, amount: u64) {
        self.balances.insert(address, amount);
    }
    
    pub fn get_nonce(&self, address: &str) -> u64 {
        *self.nonces.get(address).unwrap_or(&0)
    }
    
    pub fn increment_nonce(&mut self, address: &str) {
        let current = self.get_nonce(address);
        self.nonces.insert(address.to_string(), current + 1);
    }
    
    /// Atomically apply transaction to state
    /// Economic model: sender mines (does work), receiver optionally pays gas
    pub fn apply_transaction(&mut self, tx: &Transaction) -> bool {
        // Check nonce
        if tx.nonce != self.get_nonce(&tx.sender) {
            return false;
        }
        
        // Check sender has enough to send the amount
        if self.get_balance(&tx.sender) < tx.amount {
            return false;
        }
        
        // Check receiver can afford gas (if gas is specified)
        let gas_cost = tx.calculate_gas_cost();
        if gas_cost > 0 && self.get_balance(&tx.receiver) < gas_cost {
            return false;
        }
        
        // Execute atomically
        let sender_balance = self.get_balance(&tx.sender);
        let receiver_balance = self.get_balance(&tx.receiver);
        
        // Sender sends amount (mining work done by sender)
        self.balances.insert(tx.sender.clone(), sender_balance - tx.amount);
        
        // Receiver receives amount minus gas (if gas is specified)
        let net_received = if gas_cost > 0 {
            receiver_balance + tx.amount - gas_cost
        } else {
            receiver_balance + tx.amount
        };
        self.balances.insert(tx.receiver.clone(), net_received);
        
        // Increment sender nonce (sender does the work)
        self.increment_nonce(&tx.sender);
        
        true
    }
    
    pub fn hash(&self) -> String {
        let data = serde_json::to_string(self).unwrap();
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hex::encode(hasher.finalize())
    }
}

/// Block with recursive proof (Mina-like)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub height: u64,
    pub timestamp: i64,
    pub transactions: Vec<Transaction>,
    pub previous_hash: String,
    pub state_hash: String,
    pub proof: Option<String>,
    pub block_hash: String,
}

impl Block {
    pub fn new(
        height: u64,
        transactions: Vec<Transaction>,
        previous_hash: String,
        state_hash: String,
    ) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        let mut block = Block {
            height,
            timestamp,
            transactions,
            previous_hash,
            state_hash,
            proof: None,
            block_hash: String::new(),
        };
        
        block.block_hash = block.compute_hash();
        block
    }
    
    pub fn compute_hash(&self) -> String {
        let tx_hashes: Vec<String> = self.transactions.iter().map(|t| t.hash()).collect();
        let data = format!(
            "{}{}{:?}{}{}{:?}",
            self.height, self.timestamp, tx_hashes, self.previous_hash, self.state_hash, self.proof
        );
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hex::encode(hasher.finalize())
    }
    
    pub fn with_proof(mut self, proof: String) -> Self {
        self.proof = Some(proof);
        self.block_hash = self.compute_hash();
        self
    }
}

/// Recursive blockchain with atomic transactions
pub struct Blockchain {
    pub chain: Vec<Block>,
    pub state: State,
    pub transaction_pool: Vec<Transaction>,
    pub gas_price: u64,
    pub min_gas_price: u64,
}

impl Blockchain {
    pub fn new() -> Self {
        let mut blockchain = Blockchain {
            chain: Vec::new(),
            state: State::new(),
            transaction_pool: Vec::new(),
            gas_price: 1,
            min_gas_price: 0,
        };
        
        blockchain.create_genesis_block();
        blockchain
    }
    
    fn create_genesis_block(&mut self) {
        // Initialize with some accounts
        self.state.set_balance("genesis".to_string(), 1_000_000);
        self.state.set_balance("alice".to_string(), 1_000);
        self.state.set_balance("bob".to_string(), 1_000);
        
        let genesis = Block::new(
            0,
            vec![],
            "0".repeat(64),
            self.state.hash(),
        ).with_proof("genesis_proof".to_string());
        
        self.chain.push(genesis);
    }
    
    pub fn get_latest_block(&self) -> &Block {
        self.chain.last().unwrap()
    }
    
    pub fn get_block(&self, height: u64) -> Option<&Block> {
        if height < self.chain.len() as u64 {
            self.chain.get(height as usize)
        } else {
            None
        }
    }
    
    /// Add transaction to pool with immediate validation
    pub fn add_transaction(&mut self, tx: Transaction) -> bool {
        if !self.validate_transaction(&tx) {
            return false;
        }
        
        self.transaction_pool.push(tx);
        true
    }
    
    fn validate_transaction(&self, tx: &Transaction) -> bool {
        // Check nonce - allow future nonces in pool (strict validation at block creation)
        if tx.nonce < self.state.get_nonce(&tx.sender) {
            return false;
        }
        
        // Check sender has enough to send the amount
        if self.state.get_balance(&tx.sender) < tx.amount {
            return false;
        }
        
        // Check receiver can afford gas (if gas is specified)
        let gas_cost = tx.calculate_gas_cost();
        if gas_cost > 0 && self.state.get_balance(&tx.receiver) < gas_cost {
            return false;
        }
        
        // If gas is specified, check gas price
        if !tx.is_gasless() {
            if let Some(gp) = tx.gas_price {
                if gp < self.min_gas_price {
                    return false;
                }
            }
        }
        
        true
    }
    
    /// Create new block with atomic transaction execution
    pub fn create_block(&mut self, transactions: Vec<Transaction>) -> Option<Block> {
        // Create a copy of state for testing
        let mut test_state = State {
            balances: self.state.balances.clone(),
            nonces: self.state.nonces.clone(),
            contracts: self.state.contracts.clone(),
        };
        
        // Try to apply all transactions atomically
        let mut valid_txs = Vec::new();
        for tx in &transactions {
            if test_state.apply_transaction(tx) {
                valid_txs.push(tx.clone());
            } else {
                // If any transaction fails, rollback all
                return None;
            }
        }
        
        // All transactions valid - create block
        let new_block = Block::new(
            self.get_latest_block().height + 1,
            valid_txs.clone(),
            self.get_latest_block().block_hash.clone(),
            test_state.hash(),
        ).with_proof(self.generate_proof(&test_state));
        
        // Update actual state
        self.state = test_state;
        self.chain.push(new_block.clone());
        
        // Remove from pool
        self.transaction_pool.retain(|tx| !valid_txs.iter().any(|v| v.tx_id == tx.tx_id));
        
        Some(new_block)
    }
    
    fn generate_proof(&self, state: &State) -> String {
        // In a real implementation, this would generate a zk-SNARK proof
        // For this micro version, we use a hash as a placeholder
        let mut rng = rand::thread_rng();
        let random: u64 = rng.gen();
        
        let data = format!(
            "{}{}{}{}",
            state.hash(),
            self.get_latest_block().height + 1,
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
            random
        );
        
        let mut hasher = Sha256::new();
        hasher.update(data.as_bytes());
        hex::encode(hasher.finalize())
    }
    
    pub fn set_gas_price(&mut self, price: u64) {
        self.gas_price = price.max(self.min_gas_price);
    }
    
    pub fn get_gas_price(&self) -> u64 {
        self.gas_price
    }
}

impl Default for Blockchain {
    fn default() -> Self {
        Self::new()
    }
}
