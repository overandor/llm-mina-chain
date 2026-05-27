use crate::protocol::{AgentId, CanonicalReceipt, ReceiptType, hash_json_canonical};
use crate::solana_agent::{QueryEngine, SolanaKnowledgeBase, SolanaRpcClient};

use super::context::RuntimeContext;
use super::query_router::QueryRouter;
use super::session_memory::{ActionBuilder, SessionMemory};
use super::types::{ActionStatus, ExecutionPlan, IntentType, ParsedIntent, PlanStep, Subsystem};

/// The Orchestrator is the central dispatcher of the semantic runtime.
///
/// It receives natural language intents, routes them to subsystems,
/// records actions in session memory, and produces canonical receipts.
pub struct Orchestrator {
    pub router: QueryRouter,
    pub memory: SessionMemory,
    pub context: RuntimeContext,
    pub client: SolanaRpcClient,
    pub engine: QueryEngine,
    pub kb: SolanaKnowledgeBase,
}

impl Orchestrator {
    pub fn new(endpoint: Option<String>) -> Self {
        let client = SolanaRpcClient::new(endpoint);
        let engine = QueryEngine::new(client.clone());
        Self {
            router: QueryRouter::new(),
            memory: SessionMemory::new(10_000),
            context: RuntimeContext::default(),
            client,
            engine,
            kb: SolanaKnowledgeBase::new(),
        }
    }

    /// Execute a single intent and return a human-readable result.
    pub async fn execute(&self, input: &str) -> String {
        let intent = self.router.parse(input);
        let plan = self.build_plan(&intent);

        // Record start
        let start_action = ActionBuilder::new(&intent.raw)
            .subsystem(self.intent_to_subsystem(&intent))
            .status(ActionStatus::Running)
            .build();
        self.memory.record(start_action.clone());

        let result = match intent.intent_type {
            IntentType::QuerySolanaAccount => self.handle_account_query(&intent).await,
            IntentType::QuerySolanaTransaction => self.handle_transaction_query(&intent).await,
            IntentType::QuerySolanaBlock => self.handle_block_query(&intent).await,
            IntentType::QuerySolanaStatus => self.handle_status_query(&intent).await,
            IntentType::QueryKnowledgeBase => self.handle_knowledge_query(&intent).await,
            IntentType::QueryFilesystem => self.handle_filesystem_query(&intent).await,
            IntentType::QueryGitHub => self.handle_github_query(&intent).await,
            IntentType::ExecuteCommand => self.handle_command(&intent).await,
            IntentType::AnalyzeWallet => self.handle_wallet_analysis(&intent).await,
            IntentType::DeployProgram => "Error: DeployProgram not yet implemented.".to_string(),
            IntentType::Unknown => self.handle_unknown(&intent).await,
        };

        // Record completion with receipt
        let receipt = CanonicalReceipt::new(
            self.intent_to_agent(&intent),
            self.intent_to_receipt_type(&intent),
            &intent.raw,
            serde_json::json!({
                "intent": intent,
                "plan": plan,
                "result": &result,
            }),
        );
        let receipt_hash = hash_json_canonical(&receipt.payload);

        let final_action = ActionBuilder::new(&intent.raw)
            .subsystem(self.intent_to_subsystem(&intent))
            .status(if result.starts_with("Error") {
                ActionStatus::Failed
            } else {
                ActionStatus::Success
            })
            .result(&result)
            .receipt_hash(receipt_hash)
            .metadata("receipt_type", format!("{:?}", receipt.receipt_type))
            .build();
        self.memory.record(final_action);

        result
    }

    fn build_plan(&self, intent: &ParsedIntent) -> ExecutionPlan {
        let steps = match intent.intent_type {
            IntentType::QuerySolanaAccount => vec![
                PlanStep {
                    step_number: 1,
                    description: "Parse pubkey from intent".to_string(),
                    subsystem: Subsystem::SolanaQuery,
                    command: "extract_pubkey".to_string(),
                    depends_on: vec![],
                },
                PlanStep {
                    step_number: 2,
                    description: "Call getAccountInfo RPC".to_string(),
                    subsystem: Subsystem::SolanaQuery,
                    command: "getAccountInfo".to_string(),
                    depends_on: vec![1],
                },
            ],
            IntentType::QueryKnowledgeBase => vec![
                PlanStep {
                    step_number: 1,
                    description: "Search knowledge base for matching topic".to_string(),
                    subsystem: Subsystem::KnowledgeBase,
                    command: "kb.ask".to_string(),
                    depends_on: vec![],
                },
            ],
            _ => vec![PlanStep {
                step_number: 1,
                description: format!("Execute {:?}", intent.intent_type),
                subsystem: self.intent_to_subsystem(intent),
                command: intent.raw.clone(),
                depends_on: vec![],
            }],
        };

        ExecutionPlan {
            intent: intent.clone(),
            steps,
        }
    }

    // ---- Handlers ----

    async fn handle_account_query(&self, intent: &ParsedIntent) -> String {
        let pubkey = intent
            .entities
            .iter()
            .find(|e| matches!(e.entity_type, super::types::EntityType::Pubkey))
            .map(|e| e.value.clone());

        match pubkey {
            Some(pk) => match self.client.get_account_info(&pk, "confirmed").await {
                Ok(v) => {
                    // Try semantic decode first
                    use crate::solana_agent::decoder;
                    if let Some(decoded) = decoder::decode_account_info(&pk, &v) {
                        serde_json::to_string_pretty(&decoded).unwrap_or_else(|e| e.to_string())
                    } else {
                        serde_json::to_string_pretty(&v).unwrap_or_else(|e| e.to_string())
                    }
                }
                Err(e) => format!("Error: {}", e),
            },
            None => "Error: No pubkey found in query. Try: 'account <pubkey>'".to_string(),
        }
    }

    async fn handle_transaction_query(&self, intent: &ParsedIntent) -> String {
        let sig = intent
            .entities
            .iter()
            .find(|e| matches!(e.entity_type, super::types::EntityType::Signature))
            .map(|e| e.value.clone());

        match sig {
            Some(s) => match self.client.get_transaction(&s, "confirmed").await {
                Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|e| e.to_string()),
                Err(e) => format!("Error: {}", e),
            },
            None => "Error: No signature found. Try: 'tx <signature>'".to_string(),
        }
    }

    async fn handle_block_query(&self, intent: &ParsedIntent) -> String {
        let slot = intent
            .entities
            .iter()
            .find(|e| matches!(e.entity_type, super::types::EntityType::Slot))
            .and_then(|e| e.value.parse().ok());

        let slot = match slot {
            Some(s) => s,
            None => match self.client.get_slot("confirmed").await {
                Ok(s) => s,
                Err(e) => return format!("Error: {}", e),
            },
        };

        match self.client.get_block(slot, "confirmed").await {
            Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {}", e),
        }
    }

    async fn handle_status_query(&self, intent: &ParsedIntent) -> String {
        let query = intent.raw.clone();
        match self.engine.execute(&query, None).await {
            Ok(result) => format!(
                "Columns: {:?}\nRows: {}\nTime: {:.2}ms",
                result.columns, result.row_count, result.execution_time_ms
            ),
            Err(e) => format!("Error: {}", e),
        }
    }

    async fn handle_knowledge_query(&self, intent: &ParsedIntent) -> String {
        match self.kb.ask(&intent.raw) {
            Some(ans) => {
                let mut out = ans.answer.clone();
                if !ans.sources.is_empty() {
                    out.push_str(&format!("\n\nSources: {}", ans.sources.join(", ")));
                }
                out
            }
            None => {
                "I don't have a specific answer for that.\n".to_string()
                    + "Try asking about: architecture, accounts, transactions, programs, tokens, staking, consensus, fees, PDAs, state compression, or security."
            }
        }
    }

    async fn handle_filesystem_query(&self, _intent: &ParsedIntent) -> String {
        format!(
            "Current directory: {}\nProject root: {:?}",
            self.context.fs.current_dir, self.context.fs.project_root
        )
    }

    async fn handle_github_query(&self, _intent: &ParsedIntent) -> String {
        if !self.context.git.is_repo {
            return "Not in a Git repository.".to_string();
        }
        format!(
            "Branch: {}\nLast commit: {}\nRemote: {:?}\nUncommitted: {:?}",
            self.context.git.branch,
            &self.context.git.last_commit[..8.min(self.context.git.last_commit.len())],
            self.context.git.remote_url,
            self.context.git.uncommitted_changes.len()
        )
    }

    async fn handle_command(&self, intent: &ParsedIntent) -> String {
        let cmd = intent
            .entities
            .iter()
            .find(|e| matches!(e.entity_type, super::types::EntityType::Command))
            .map(|e| e.value.clone())
            .unwrap_or_default();

        if cmd.is_empty() {
            return "Error: No command provided. Use !<command> or run <command>".to_string();
        }

        match std::process::Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                format!("stdout:\n{}\nstderr:\n{}", stdout, stderr)
            }
            Err(e) => format!("Error executing command: {}", e),
        }
    }

    async fn handle_wallet_analysis(&self, intent: &ParsedIntent) -> String {
        let pubkey = intent
            .entities
            .iter()
            .find(|e| matches!(e.entity_type, super::types::EntityType::Pubkey))
            .map(|e| e.value.clone());

        match pubkey {
            Some(pk) => {
                let mut result = String::new();
                match self.client.get_balance(&pk, "confirmed").await {
                    Ok(lamports) => {
                        result.push_str(&format!(
                            "Balance: {:.9} SOL ({} lamports)\n",
                            lamports as f64 / 1e9,
                            lamports
                        ));
                    }
                    Err(e) => result.push_str(&format!("Balance error: {}\n", e)),
                }
                match self
                    .client
                    .get_token_accounts_by_owner(&pk, None, "confirmed")
                    .await
                {
                    Ok(v) => {
                        result.push_str(&format!(
                            "Token accounts: {}\n",
                            serde_json::to_string_pretty(&v).unwrap_or_default()
                        ));
                    }
                    Err(e) => result.push_str(&format!("Token accounts error: {}\n", e)),
                }
                result
            }
            None => "Error: No wallet pubkey provided.".to_string(),
        }
    }

    async fn handle_unknown(&self, _intent: &ParsedIntent) -> String {
        "I'm not sure how to handle that. Try 'help' for available commands, or rephrase your question.".to_string()
    }

    // ---- Mappings ----

    fn intent_to_subsystem(&self, intent: &ParsedIntent) -> Subsystem {
        match intent.intent_type {
            IntentType::QuerySolanaAccount
            | IntentType::QuerySolanaTransaction
            | IntentType::QuerySolanaBlock
            | IntentType::QuerySolanaStatus
            | IntentType::AnalyzeWallet => Subsystem::SolanaQuery,
            IntentType::QueryKnowledgeBase => Subsystem::KnowledgeBase,
            IntentType::QueryFilesystem => Subsystem::LocalFilesystem,
            IntentType::QueryGitHub => Subsystem::GitHub,
            IntentType::ExecuteCommand => Subsystem::Terminal,
            IntentType::DeployProgram => Subsystem::ProofSystem,
            IntentType::Unknown => Subsystem::KnowledgeBase,
        }
    }

    fn intent_to_agent(&self, intent: &ParsedIntent) -> AgentId {
        match intent.intent_type {
            IntentType::QuerySolanaAccount
            | IntentType::QuerySolanaTransaction
            | IntentType::QuerySolanaBlock
            | IntentType::QuerySolanaStatus
            | IntentType::AnalyzeWallet => AgentId::SolanaQuery,
            IntentType::QueryKnowledgeBase
            | IntentType::QueryFilesystem
            | IntentType::QueryGitHub
            | IntentType::ExecuteCommand
            | IntentType::Unknown => AgentId::SolanaQuery,
            IntentType::DeployProgram => AgentId::ProofProvenance,
        }
    }

    fn intent_to_receipt_type(&self, intent: &ParsedIntent) -> ReceiptType {
        match intent.intent_type {
            IntentType::QuerySolanaAccount
            | IntentType::QuerySolanaTransaction
            | IntentType::QuerySolanaBlock
            | IntentType::QuerySolanaStatus
            | IntentType::AnalyzeWallet => ReceiptType::QueryResult,
            IntentType::QueryKnowledgeBase => ReceiptType::QueryResult,
            IntentType::QueryFilesystem | IntentType::QueryGitHub | IntentType::ExecuteCommand => {
                ReceiptType::Audit
            }
            IntentType::DeployProgram => ReceiptType::Proof,
            IntentType::Unknown => ReceiptType::Audit,
        }
    }
}
