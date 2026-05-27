//! Daily Integration Checkpoint Tests
//!
//! These tests verify that all three agent territories can coexist in a single
//! compilation unit, share canonical types, and produce deterministic outputs.
//!
//! Run: cargo test --test daily_integration

use llm_mina_chain::protocol::{
    AgentId, CanonicalConfig, CanonicalLogEntry, CanonicalReceipt, DeterministicHash, LogLevel,
    ReceiptType, SemVer, hash_json_canonical, merkle_root_from_hashes,
};
use llm_mina_chain::{
    Blockchain, Transaction, State,
    ProofSystem,
};
use llm_mina_chain::solana_agent::{SolanaRpcClient, QueryEngine, SolanaKnowledgeBase};
#[cfg(feature = "semantic")]
use llm_mina_chain::semantic_runtime::{Orchestrator, QueryRouter, RuntimeContext, SessionMemory};

/// 1. All agent territories compile together.
#[test]
fn test_all_modules_compile_and_link() {
    // Agent 1 territory
    let _bc = Blockchain::new();
    let _state = State::new();

    // Agent 2 territory
    let _client = SolanaRpcClient::new(None);
    let _engine = QueryEngine::new(SolanaRpcClient::new(None));
    let _kb = SolanaKnowledgeBase::new();

    // Agent 3 territory
    // ProofSystem may require feature flags; we just verify the type exists
    // by referencing it from the protocol layer.
    let _ = ProofSystem::new();
}

/// 2. Protocol types are deterministic.
#[test]
fn test_semver_ordering() {
    let v1 = SemVer::new(0, 1, 0);
    let v2 = SemVer::new(0, 1, 1);
    let v3 = SemVer::new(0, 2, 0);
    assert!(v1 < v2);
    assert!(v2 < v3);
    assert_eq!(v1.to_string(), "0.1.0");
}

/// 3. Receipt creation and integrity verification.
#[test]
fn test_receipt_lifecycle() {
    let payload = serde_json::json!({"action": "transfer", "amount": 100});
    let receipt = CanonicalReceipt::new(
        AgentId::CoreRuntime,
        ReceiptType::Execution,
        "test execution",
        payload,
    );
    assert!(receipt.verify_integrity());
    assert_eq!(receipt.source, AgentId::CoreRuntime);
    assert_eq!(receipt.receipt_type, ReceiptType::Execution);
    assert_eq!(receipt.version, SemVer::CURRENT);
}

/// 4. Receipts with Merkle root and signature.
#[test]
fn test_receipt_with_merkle_and_sig() {
    let payload = serde_json::json!({"slot": 42});
    let receipt = CanonicalReceipt::new(
        AgentId::SolanaQuery,
        ReceiptType::QueryResult,
        "slot query",
        payload,
    )
    .with_merkle_root([0u8; 32])
    .with_signature(vec![1, 2, 3]);

    assert!(receipt.merkle_root.is_some());
    assert!(receipt.signature.is_some());
    assert!(receipt.verify_integrity());
}

/// 5. Merkle root computation is deterministic.
#[test]
fn test_merkle_root_determinism() {
    let a = [1u8; 32];
    let b = [2u8; 32];
    let root1 = merkle_root_from_hashes(&[a, b]);
    let root2 = merkle_root_from_hashes(&[a, b]);
    assert_eq!(root1, root2);
}

/// 6. Hash function is deterministic.
#[test]
fn test_hash_json_canonical_determinism() {
    let value = serde_json::json!({"a": 1, "b": 2});
    let h1 = hash_json_canonical(&value);
    let h2 = hash_json_canonical(&value);
    assert_eq!(h1, h2);
}

/// 7. Config loads from environment without panic.
#[test]
fn test_config_from_env_or_default() {
    let config = CanonicalConfig::from_env_or_default();
    assert_eq!(config.protocol_version, SemVer::CURRENT);
    // Verify critical fields are populated
    assert!(!config.solana.rpc_endpoint.is_empty());
    assert!(!config.api.bind_addr.is_empty());
    assert!(config.core.deterministic_mode);
}

/// 8. Structured logging produces valid JSON.
#[test]
fn test_canonical_log_json() {
    let entry = CanonicalLogEntry::new(
        LogLevel::Info,
        AgentId::ProofProvenance,
        "test_module",
        "test_event",
        serde_json::json!({"key": "value"}),
        "trace-123",
    );
    let json_str = entry.to_canonical_json();
    let parsed: serde_json::Value = serde_json::from_str(&json_str).expect("log must be valid JSON");
    assert_eq!(parsed["level"], "Info");
    assert_eq!(parsed["agent"], "proof_provenance");
    assert_eq!(parsed["event"], "test_event");
    assert_eq!(parsed["trace_id"], "trace-123");
}

/// 9. Cross-module receipt: Core runtime produces a receipt for a block.
#[test]
fn test_cross_module_receipt_block() {
    let mut bc = Blockchain::new();
    let tx = Transaction::new(
        "alice".to_string(),
        "bob".to_string(),
        100,
        0,
        None,
        None,
    );
    bc.add_transaction(tx.clone());
    let block = bc.create_block(vec![tx]).expect("block must be created");

    let payload = serde_json::json!({
        "block_hash": block.block_hash,
        "height": block.height,
        "tx_count": block.transactions.len(),
    });
    let receipt = CanonicalReceipt::new(
        AgentId::CoreRuntime,
        ReceiptType::Execution,
        "block produced",
        payload,
    );
    assert!(receipt.verify_integrity());
}

/// 10. Cross-module receipt: Solana query result.
#[test]
fn test_cross_module_receipt_query() {
    let payload = serde_json::json!({
        "query": "SELECT * FROM status",
        "status": "ok",
    });
    let receipt = CanonicalReceipt::new(
        AgentId::SolanaQuery,
        ReceiptType::QueryResult,
        "sql query executed",
        payload,
    );
    assert!(receipt.verify_integrity());
}

/// 11. Cross-module receipt: Proof generation.
#[test]
fn test_cross_module_receipt_proof() {
    let payload = serde_json::json!({
        "circuit": "state_transition",
        "verified": true,
    });
    let receipt = CanonicalReceipt::new(
        AgentId::ProofProvenance,
        ReceiptType::Proof,
        "zk proof generated",
        payload,
    );
    assert!(receipt.verify_integrity());
}

/// 12. No duplicate type definitions across modules.
/// This is enforced at compile time by the imports above.
/// If any module redefines `DeterministicHash`, `SemVer`, etc.,
/// this test file will fail to compile.
#[test]
fn test_no_duplicate_types_at_compile_time() {
    // This test exists purely to force compilation of all cross-module imports.
    // If different modules define conflicting types, rustc will fail here.
    let _: DeterministicHash = [0u8; 32];
    let _: SemVer = SemVer::CURRENT;
    let _: AgentId = AgentId::CoreRuntime;
}

/// 13. Semantic runtime query router parses intents correctly.
#[cfg(feature = "semantic")]
#[test]
fn test_query_router_account() {
    let router = QueryRouter::new();
    let intent = router.parse("get account info for So11111111111111111111111111111111111111112");
    assert_eq!(intent.intent_type, llm_mina_chain::semantic_runtime::IntentType::QuerySolanaAccount);
    assert_eq!(intent.entities.len(), 1);
}

/// 14. Semantic runtime session memory records actions.
#[cfg(feature = "semantic")]
#[test]
fn test_session_memory() {
    let mem = SessionMemory::new(100);
    assert!(mem.is_empty());
    let action = llm_mina_chain::semantic_runtime::ActionBuilder::new("test query")
        .status(llm_mina_chain::semantic_runtime::ActionStatus::Success)
        .result("ok")
        .build();
    mem.record(action);
    assert_eq!(mem.len(), 1);
    let last = mem.get_last_n(1);
    assert_eq!(last[0].result_summary, "ok");
}

/// 15. Runtime context can be gathered without panic.
#[cfg(feature = "semantic")]
#[test]
fn test_runtime_context_gather() {
    let ctx = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(RuntimeContext::gather());
    assert!(!ctx.fs.current_dir.is_empty());
}

/// 16. Orchestrator can be instantiated.
#[cfg(feature = "semantic")]
#[test]
fn test_orchestrator_instantiation() {
    let orch = Orchestrator::new(None);
    assert!(orch.memory.is_empty());
}
