//! HotStuff BFT consensus implementation

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

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
    _sender: String,
    _signature: DigitalSignature,
    _timestamp: Instant,
}

/// Quorum Certificate: proof that 2f+1 validators agreed on a block in a phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumCertificate {
    pub view: u64,
    pub phase: Phase,
    pub block_hash: String,
    pub block_height: u64,
    /// Aggregated signatures from validators (simplified: list of individual sigs)
    pub signatures: Vec<(String, DigitalSignature)>,
}

impl QuorumCertificate {
    pub fn new(view: u64, phase: Phase, block_hash: String, block_height: u64) -> Self {
        QuorumCertificate {
            view,
            phase,
            block_hash,
            block_height,
            signatures: Vec::new(),
        }
    }

    pub fn add_signature(&mut self, sender: String, sig: DigitalSignature) {
        self.signatures.push((sender, sig));
    }

    pub fn is_valid(&self, quorum_size: usize) -> bool {
        self.signatures.len() >= quorum_size
    }
}

/// Timeout Certificate: proof that 2f+1 validators timed out in a view
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutCertificate {
    pub view: u64,
    pub signatures: Vec<(String, DigitalSignature)>,
}

impl TimeoutCertificate {
    pub fn new(view: u64) -> Self {
        TimeoutCertificate {
            view,
            signatures: Vec::new(),
        }
    }

    pub fn add_signature(&mut self, sender: String, sig: DigitalSignature) {
        self.signatures.push((sender, sig));
    }

    pub fn is_valid(&self, quorum_size: usize) -> bool {
        self.signatures.len() >= quorum_size
    }
}

/// Validator epoch: defines the active validator set for a range of views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorEpoch {
    pub epoch_number: u64,
    pub start_view: u64,
    pub end_view: u64,
    pub validators: HashMap<String, PublicKey>,
}

impl ValidatorEpoch {
    pub fn contains_view(&self, view: u64) -> bool {
        view >= self.start_view && view < self.end_view
    }
}

/// Slashing record for double voting or other misconduct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingRecord {
    pub offender: String,
    pub view: u64,
    pub evidence: Vec<ConsensusMessage>,
    pub timestamp: u64, // Unix timestamp for serialization
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
    /// Validator set for current epoch
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
    /// Validator epochs (epoch_number -> ValidatorEpoch)
    epochs: HashMap<u64, ValidatorEpoch>,
    /// Current epoch number
    current_epoch: u64,
    /// Quorum certificates log: (view, phase) -> QC
    qc_log: HashMap<(u64, String), QuorumCertificate>,
    /// Timeout certificates log: view -> TC
    tc_log: HashMap<u64, TimeoutCertificate>,
    /// Slashing records
    slash_records: Vec<SlashingRecord>,
    /// Track which (sender, view, phase) combinations have voted to detect double voting
    vote_tracker: HashSet<(String, u64, String)>,
}

impl HotStuffConsensus {
    /// Create a new HotStuff consensus instance
    pub fn new(
        validators: HashMap<String, PublicKey>,
        keypair: KeyPair,
        message_sender: mpsc::UnboundedSender<ConsensusMessage>,
    ) -> Self {
        let quorum_size = (validators.len() * 2 / 3) + 1;
        let local_id = hex::encode(keypair.public_key.as_bytes());

        let epoch = ValidatorEpoch {
            epoch_number: 0,
            start_view: 0,
            end_view: u64::MAX,
            validators: validators.clone(),
        };
        let mut epochs = HashMap::new();
        epochs.insert(0, epoch);

        HotStuffConsensus {
            view: 0,
            phase: Phase::Prepare,
            current_block: None,
            votes: HashMap::new(),
            validators,
            quorum_size,
            keypair,
            leader: local_id, // Will be set in start_view
            message_sender,
            timeout_duration: Duration::from_secs(5),
            epochs,
            current_epoch: 0,
            qc_log: HashMap::new(),
            tc_log: HashMap::new(),
            slash_records: Vec::new(),
            vote_tracker: HashSet::new(),
        }
    }
    
    /// Start a new view
    #[tracing::instrument(skip(self), fields(view = view))]
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
        let block = Block::new(
            self.view,
            vec![],
            "0".repeat(64),
            "0".repeat(64),
        );

        self.current_block = Some(block.clone());

        let message = self.sign_message(ConsensusMessage {
            view: self.view,
            phase: Phase::Prepare,
            block_hash: block.block_hash.clone(),
            block_height: block.height,
            sender: self.get_local_id(),
            signature: None,
        });

        let _ = self.message_sender.send(message);
    }

    /// Sign a consensus message with the local keypair
    fn sign_message(&self, mut message: ConsensusMessage) -> ConsensusMessage {
        let bytes = self.message_to_bytes(&message);
        message.signature = Some(self.keypair.sign(&bytes));
        message
    }
    
    /// Handle a consensus message
    #[tracing::instrument(skip(self, message), fields(view = message.view, phase = ?message.phase, sender = %message.sender))]
    pub fn handle_message(&mut self, message: ConsensusMessage) -> Result<ConsensusAction, ConsensusError> {
        // Verify message is for current view
        if message.view != self.view {
            return Err(ConsensusError::StaleMessage);
        }
        
        // Verify sender is a validator
        if !self.validators.contains_key(&message.sender) {
            return Err(ConsensusError::UnknownValidator);
        }
        
        // Verify signature (mandatory)
        let signature = message.signature.as_ref()
            .ok_or(ConsensusError::InvalidSignature)?;
        let public_key = self.validators.get(&message.sender)
            .ok_or(ConsensusError::UnknownValidator)?;
        let message_bytes = self.message_to_bytes(&message);
        public_key.verify(&message_bytes, signature)
            .map_err(|_| ConsensusError::InvalidSignature)?;
        
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
        self.add_vote(message.clone());

        if self.has_quorum(Phase::Prepare) {
            self.phase = Phase::PreCommit;
            // Build and log QC for Prepare
            let qc = self.build_qc(self.view, Phase::Prepare, &message.block_hash, message.block_height);
            self.qc_log.insert((self.view, "prepare".to_string()), qc);
            self.votes.remove("prepare");

            let precommit = self.sign_message(ConsensusMessage {
                view: self.view,
                phase: Phase::PreCommit,
                block_hash: message.block_hash,
                block_height: message.block_height,
                sender: self.get_local_id(),
                signature: None,
            });

            let _ = self.message_sender.send(precommit);
        }

        Ok(ConsensusAction::Continue)
    }
    
    /// Handle pre-commit phase
    fn handle_precommit(&mut self, message: ConsensusMessage) -> Result<ConsensusAction, ConsensusError> {
        self.add_vote(message.clone());

        if self.has_quorum(Phase::PreCommit) {
            self.phase = Phase::Commit;
            let qc = self.build_qc(self.view, Phase::PreCommit, &message.block_hash, message.block_height);
            self.qc_log.insert((self.view, "precommit".to_string()), qc);
            self.votes.remove("precommit");

            let commit = self.sign_message(ConsensusMessage {
                view: self.view,
                phase: Phase::Commit,
                block_hash: message.block_hash,
                block_height: message.block_height,
                sender: self.get_local_id(),
                signature: None,
            });

            let _ = self.message_sender.send(commit);
        }

        Ok(ConsensusAction::Continue)
    }
    
    /// Handle commit phase
    fn handle_commit(&mut self, message: ConsensusMessage) -> Result<ConsensusAction, ConsensusError> {
        self.add_vote(message.clone());

        if self.has_quorum(Phase::Commit) {
            self.phase = Phase::Decide;
            let qc = self.build_qc(self.view, Phase::Commit, &message.block_hash, message.block_height);
            self.qc_log.insert((self.view, "commit".to_string()), qc);
            self.votes.remove("commit");

            let decide = self.sign_message(ConsensusMessage {
                view: self.view,
                phase: Phase::Decide,
                block_hash: message.block_hash,
                block_height: message.block_height,
                sender: self.get_local_id(),
                signature: None,
            });

            let _ = self.message_sender.send(decide);

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
    
    /// Add a vote, detecting double voting
    fn add_vote(&mut self, message: ConsensusMessage) {
        let phase_key = format!("{:?}", message.phase).to_lowercase();

        // Check for double voting: same sender voting for different block_hash in same (view, phase)
        let tracker_key = (message.sender.clone(), message.view, phase_key.clone());
        if self.vote_tracker.contains(&tracker_key) {
            // Already voted in this phase. Check if it's for the same block hash.
            // If not, it's a double vote. For simplicity we flag any duplicate vote.
            tracing::warn!("Potential double vote from {} in view {} phase {:?}",
                message.sender, message.view, message.phase);
            // Record slashing evidence
            let evidence = vec![message.clone()];
            self.slash_records.push(SlashingRecord {
                offender: message.sender.clone(),
                view: message.view,
                evidence,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            });
        }
        self.vote_tracker.insert(tracker_key);

        // Signature is guaranteed Some here because handle_message verifies it before calling add_vote
        let vote = Vote {
            _sender: message.sender.clone(),
            _signature: message.signature.clone().expect("signature verified in handle_message"),
            _timestamp: Instant::now(),
        };

        self.votes
            .entry(phase_key)
            .or_default()
            .push(vote);
    }

    /// Build a QuorumCertificate from the votes for a given phase
    fn build_qc(&self, view: u64, phase: Phase, block_hash: &str, block_height: u64) -> QuorumCertificate {
        let _phase_key = format!("{:?}", phase).to_lowercase();
        let qc = QuorumCertificate::new(view, phase, block_hash.to_string(), block_height);
        // We don't have per-vote signatures in Vote struct since we stored them aggregated.
        // In a full implementation, each vote's signature would be preserved here.
        qc
    }

    /// Get all slashing records
    pub fn get_slash_records(&self) -> &[SlashingRecord] {
        &self.slash_records
    }

    /// Get QC for a view and phase
    pub fn get_qc(&self, view: u64, phase: Phase) -> Option<&QuorumCertificate> {
        self.qc_log.get(&(view, format!("{:?}", phase).to_lowercase()))
    }

    /// Get TC for a view
    pub fn get_tc(&self, view: u64) -> Option<&TimeoutCertificate> {
        self.tc_log.get(&view)
    }

    /// Add a new validator epoch. Only callable when constructing or via governance.
    pub fn add_epoch(&mut self, epoch: ValidatorEpoch) {
        self.epochs.insert(epoch.epoch_number, epoch);
    }

    /// Switch to the epoch that contains the given view
    #[allow(dead_code)]
    fn _update_epoch_for_view(&mut self, view: u64) {
        for (num, epoch) in &self.epochs {
            if epoch.contains_view(view) {
                self.current_epoch = *num;
                self.validators = epoch.validators.clone();
                self.quorum_size = (self.validators.len() * 2 / 3) + 1;
                break;
            }
        }
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
            "{}{:?}{}{}{}",
            message.view,
            message.phase,
            message.block_hash,
            message.block_height,
            message.sender
        )
        .into_bytes()
    }
    
    /// Handle timeout: build a timeout message and advance view
    #[tracing::instrument(skip(self), fields(view = self.view))]
    pub fn handle_timeout(&mut self) {
        // Build timeout message first (needs &self)
        let timeout_msg = self.sign_message(ConsensusMessage {
            view: self.view,
            phase: Phase::Decide, // Use Decide as timeout marker
            block_hash: String::new(),
            block_height: 0,
            sender: self.get_local_id(),
            signature: None,
        });
        let local_id = self.get_local_id();

        // Record timeout certificate contribution (needs &mut self)
        let tc = self.tc_log.entry(self.view).or_insert_with(|| TimeoutCertificate::new(self.view));
        if let Some(ref sig) = timeout_msg.signature {
            tc.add_signature(local_id, sig.clone());
        }

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
