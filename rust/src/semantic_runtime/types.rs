use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A unified action represents anything the semantic runtime does.
/// It captures the intent, the subsystem that handled it, and the receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAction {
    pub id: String,
    pub timestamp: u64,
    pub intent: String,
    pub subsystem: Subsystem,
    pub status: ActionStatus,
    pub result_summary: String,
    pub receipt_hash: Option<[u8; 32]>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Subsystem {
    SolanaQuery,
    KnowledgeBase,
    LocalFilesystem,
    GitHub,
    Terminal,
    ProofSystem,
    BlockchainCore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionStatus {
    Pending,
    Running,
    Success,
    Failed,
}

/// A parsed intent from natural language or structured input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedIntent {
    pub raw: String,
    pub intent_type: IntentType,
    pub entities: Vec<Entity>,
    pub confidence: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntentType {
    QuerySolanaAccount,
    QuerySolanaTransaction,
    QuerySolanaBlock,
    QuerySolanaStatus,
    QueryKnowledgeBase,
    QueryFilesystem,
    QueryGitHub,
    ExecuteCommand,
    DeployProgram,
    AnalyzeWallet,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub entity_type: EntityType,
    pub value: String,
    pub position: (usize, usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityType {
    Pubkey,
    Signature,
    Slot,
    ProgramId,
    Mint,
    FilePath,
    GitHubRepo,
    GitHubIssue,
    Command,
    Question,
    Topic,
}

/// A plan is a sequence of steps to fulfill an intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub intent: ParsedIntent,
    pub steps: Vec<PlanStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub step_number: usize,
    pub description: String,
    pub subsystem: Subsystem,
    pub command: String,
    pub depends_on: Vec<usize>,
}
