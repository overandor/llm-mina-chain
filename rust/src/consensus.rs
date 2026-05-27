//! HotStuff BFT consensus implementation

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::timeout;

use crate::{Block, DigitalSignature, KeyPair, PublicKey};

/// Consensus phase
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Prepare,
    PreCommit,
    Commit,
    Decide,
}

/// Consensus message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusMessage {
    pub view: u64,
    pub phase: Phase,
    pub block_hash: String,
    pub block_height: u64,
    pub sender: String,
    pub signature: Option<DigitalSignature>,
}

/// Vote record
#[derive(Debug, Clone)]
struct Vote {
    sender: String,
    signature: DigitalSignature,
    timestamp: Instant,
}

/// HotStuff consensus engine
pub struct HotStuffConsensus {
    /// Current view number
    view: u64,
    /// Current phase
    phase: Phase,
    /// Current block being proposed
    current_block: Option<Block>,
    /// Votes for current phase
    votes: HashMap<String, Vec<Vote>>,
    /// Validator set
    validators: HashMap<String, PublicKey>,
    /// Quorum size (2f+1)
    quorum_size: usize,
    /// Local key pair
    keypair: KeyPair,
    /// Leader for current view
    leader: String,
    /// Message sender
    message_sender: mpsc::UnboundedSender<ConsensusMessage>,
    /// Timeout duration
    timeout_duration: Duration,
}

impl HotStuffConsensus {
    /// Create a new HotStuff consensus instance
    pub fn new(
        validators: HashMap<String, PublicKey>,
        keypair: KeyPair,
        message_sender: mpsc::UnboundedSender<ConsensusMessage>,
    ) -> Self {
        let quorum_size = (validators.len() * 2 / 3) + 1;
        
        HotStuffConsensus {
            view: 0,
            phase: Phase::Prepare,
            current_block: None,
            votes: HashMap::new(),
            validators,
            quorum_size,
            keypair,
            leader: String::new(), // Will be set in start_view
            message_sender,
            timeout_duration: Duration::from_secs(5),
        }
    }
    
    /// Start a new view
    pub fn start_view(&mut self, view: u64) {
        self.view = view;
        self.phase = Phase::Prepare;
        self.current_block = None;
        self.votes.clear();
        
        // Select leader (round-robin based on view)
        let validator_keys: Vec<String> = self.validators.keys().cloned().collect();
        self.leader = validator_keys[(view as usize) % validator_keys.len()].clone();
        
        // If we are the leader, propose a block
        if self.leader == self.get_local_id() {
            self.propose_block();
        }
    }
    
    /// Get local node ID
    fn get_local_id(&self) -> String {
        hex::encode(self.keypair.public_key.as_bytes())
    }
    
    /// Propose a new block (leader only)
    fn propose_block(&mut self) {
        // In a real implementation, this would get the next block from the blockchain
        // For now, we'll just create a placeholder
        let block = Block::new(
            0, // Will be set by blockchain
            vec![],
            "0".repeat(64),
            "0".repeat(64),
        );
        
        self.current_block = Some(block.clone());
        
        // Broadcast prepare message
        let message = ConsensusMessage {
            view: self.view,
            phase: Phase::Prepare,
            block_hash: block.block_hash.clone(),
            block_height: block.height,
            sender: self.get_local_id(),
            signature: None,
        };
        
        let _ = self.message_sender.send(message);
    }
    
    /// Handle a consensus message
    pub fn handle_message(&mut self, message: ConsensusMessage) -> Result<ConsensusAction, ConsensusError> {
        // Verify message is for current view
        if message.view != self.view {
            return Err(ConsensusError::StaleMessage);
        }
        
        // Verify sender is a validator
        if !self.validators.contains_key(&message.sender) {
            return Err(ConsensusError::UnknownValidator);
        }
        
        // Verify signature if present
        if let Some(ref signature) = message.signature {
            let public_key = self.validators.get(&message.sender).unwrap();
            let message_bytes = self.message_to_bytes(&message);
            public_key.verify(&message_bytes, signature)
                .map_err(|_| ConsensusError::InvalidSignature)?;
        }
        
        // Process based on phase
        match message.phase {
            Phase::Prepare => self.handle_prepare(message),
            Phase::PreCommit => self.handle_precommit(message),
            Phase::Commit => self.handle_commit(message),
            Phase::Decide => self.handle_decide(message),
        }
    }
    
    /// Handle prepare phase
    fn handle_prepare(&mut self, message: ConsensusMessage) -> Result<ConsensusAction, ConsensusError> {
        // Store the vote
        self.add_vote(message.clone());
        
        // Check if we have quorum
        if self.has_quorum(Phase::Prepare) {
            self.phase = Phase::PreCommit;
            self.votes.remove(&"prepare".to_string());
            
            // Broadcast pre-commit
            let precommit = ConsensusMessage {
                view: self.view,
                phase: Phase::PreCommit,
                block_hash: message.block_hash,
                block_height: message.block_height,
                sender: self.get_local_id(),
                signature: None,
            };
            
            let _ = self.message_sender.send(precommit);
        }
        
        Ok(ConsensusAction::Continue)
    }
    
    /// Handle pre-commit phase
    fn handle_precommit(&mut self, message: ConsensusMessage) -> Result<ConsensusAction, ConsensusError> {
        self.add_vote(message.clone());
        
        if self.has_quorum(Phase::PreCommit) {
            self.phase = Phase::Commit;
            self.votes.remove(&"precommit".to_string());
            
            // Broadcast commit
            let commit = ConsensusMessage {
                view: self.view,
                phase: Phase::Commit,
                block_hash: message.block_hash,
                block_height: message.block_height,
                sender: self.get_local_id(),
                signature: None,
            };
            
            let _ = self.message_sender.send(commit);
        }
        
        Ok(ConsensusAction::Continue)
    }
    
    /// Handle commit phase
    fn handle_commit(&mut self, message: ConsensusMessage) -> Result<ConsensusAction, ConsensusError> {
        self.add_vote(message.clone());
        
        if self.has_quorum(Phase::Commit) {
            self.phase = Phase::Decide;
            self.votes.remove(&"commit".to_string());
            
            // Broadcast decide
            let decide = ConsensusMessage {
                view: self.view,
                phase: Phase::Decide,
                block_hash: message.block_hash,
                block_height: message.block_height,
                sender: self.get_local_id(),
                signature: None,
            };
            
            let _ = self.message_sender.send(decide);
            
            // Return the decided block
            if let Some(ref block) = self.current_block {
                return Ok(ConsensusAction::Decide(block.clone()));
            }
        }
        
        Ok(ConsensusAction::Continue)
    }
    
    /// Handle decide phase
    fn handle_decide(&mut self, _message: ConsensusMessage) -> Result<ConsensusAction, ConsensusError> {
        // Move to next view
        self.start_view(self.view + 1);
        Ok(ConsensusAction::Continue)
    }
    
    /// Add a vote
    fn add_vote(&mut self, message: ConsensusMessage) {
        let phase_key = format!("{:?}", message.phase).to_lowercase();
        let vote = Vote {
            sender: message.sender.clone(),
            signature: message.signature.unwrap_or_else(|| {
                // Create dummy signature for now
                DigitalSignature::from_bytes([0u8; 64])
            }),
            timestamp: Instant::now(),
        };
        
        self.votes
            .entry(phase_key)
            .or_insert_with(Vec::new)
            .push(vote);
    }
    
    /// Check if we have quorum for a phase
    fn has_quorum(&self, phase: Phase) -> bool {
        let phase_key = format!("{:?}", phase).to_lowercase();
        if let Some(votes) = self.votes.get(&phase_key) {
            votes.len() >= self.quorum_size
        } else {
            false
        }
    }
    
    /// Convert message to bytes for signing
    fn message_to_bytes(&self, message: &ConsensusMessage) -> Vec<u8> {
        format!(
            "{}{}{}{}{}",
            message.view,
            format!("{:?}", message.phase),
            message.block_hash,
            message.block_height,
            message.sender
        )
        .into_bytes()
    }
    
    /// Handle timeout
    pub fn handle_timeout(&mut self) {
        // Move to next view
        self.start_view(self.view + 1);
    }
    
    /// Run the consensus timeout loop
    pub async fn run_timeout_loop(&mut self) {
        loop {
            tokio::time::sleep(self.timeout_duration).await;
            self.handle_timeout();
        }
    }
}

/// Consensus action
#[derive(Debug, Clone)]
pub enum ConsensusAction {
    Continue,
    Decide(Block),
}

/// Consensus errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusError {
    StaleMessage,
    UnknownValidator,
    InvalidSignature,
    InvalidPhase,
}

impl std::fmt::Display for ConsensusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConsensusError::StaleMessage => write!(f, "Stale message"),
            ConsensusError::UnknownValidator => write!(f, "Unknown validator"),
            ConsensusError::InvalidSignature => write!(f, "Invalid signature"),
            ConsensusError::InvalidPhase => write!(f, "Invalid phase"),
        }
    }
}

impl std::error::Error for ConsensusError {}
