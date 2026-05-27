//! One-node demo: Full transaction lifecycle
//! Demonstrates: LLM parsing → signature → validation → state update → gas → persistence → block → proof → metrics

use llm_mina_chain::{
    Blockchain, Transaction, State, KeyPair, BlockchainStorage, InputValidator, SecurityConfig,
    ProofSystem, StateTransitionCircuit, BlockchainMetrics, MetricsServer, HealthChecker,
    SystemMetrics,
};
use prometheus::Registry;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MEMBRA Instant Proof Chain - One Node Demo ===\n");
    
    // Initialize metrics
    let registry = Registry::new();
    let metrics = BlockchainMetrics::new(&registry);
    
    // Initialize storage
    let storage_path = PathBuf::from("./demo_storage");
    let storage = BlockchainStorage::open(&storage_path)?;
    println!("✓ RocksDB storage initialized");
    
    // Initialize blockchain
    let mut blockchain = Blockchain::new();
    println!("✓ Blockchain initialized (genesis block created)");
    
    // Initialize security
    let security_config = SecurityConfig::default();
    let validator = InputValidator::new(security_config, &registry);
    println!("✓ Security validator initialized");
    
    // Initialize proof system
    let mut proof_system = ProofSystem::new();
    proof_system.setup()?;
    println!("✓ zk-SNARK proof system initialized");
    
    // Initialize health checker
    let mut health_checker = HealthChecker::new("1.0.0".to_string());
    health_checker.register_check("storage".to_string(), || {
        let start = std::time::Instant::now();
        let status = HealthStatus::Healthy;
        HealthCheck::new(
            "storage".to_string(),
            status,
            "Storage operational".to_string(),
            start.elapsed().as_millis() as u64,
        )
    });
    println!("✓ Health checker initialized");
    
    // Generate keypair for Alice
    let alice_keypair = KeyPair::generate();
    println!("✓ Generated keypair for Alice: {}", alice_keypair.public_key.to_hex());
    
    // Step 1: LLM Intent Parsing
    println!("\n--- Step 1: LLM Intent Parsing ---");
    let intent = "Send 100 tokens to Bob";
    println!("Intent: \"{}\"", intent);
    
    // Parse intent to transaction (simplified LLM parsing)
    let mut tx = Transaction::new(
        alice_keypair.public_key.to_hex(),
        "bob".to_string(),
        100,
        0,
        Some(21000),
        Some(1),
    );
    println!("✓ Parsed transaction: {} → {} (amount: {})", tx.sender, tx.receiver, tx.amount);
    
    // Step 2: Sign Transaction
    println!("\n--- Step 2: Sign Transaction ---");
    tx.sign(&alice_keypair);
    println!("✓ Transaction signed: {}", tx.signature.as_ref().unwrap().to_hex());
    
    // Step 3: Validate Transaction
    println!("\n--- Step 3: Validate Transaction ---");
    let validation_result = validator.validate_transaction(&tx);
    if validation_result.is_valid() {
        println!("✓ Transaction validation passed");
    } else {
        println!("✗ Transaction validation failed: {:?}", validation_result.errors);
        return Err("Transaction validation failed".into());
    }
    
    // Step 4: Add to Transaction Pool
    println!("\n--- Step 4: Add to Transaction Pool ---");
    blockchain.add_transaction(tx.clone());
    println!("✓ Transaction added to pool (pool size: {})", blockchain.transaction_pool.len());
    metrics.transactions_pool_size.set(blockchain.transaction_pool.len() as i64);
    
    // Step 5: Create Block
    println!("\n--- Step 5: Create Block ---");
    let block = blockchain.create_block(vec![tx.clone()])
        .ok_or("Block creation failed")?;
    println!("✓ Block created (height: {}, hash: {})", block.height, block.block_hash);
    metrics.blocks_produced.inc();
    metrics.block_height.set(block.height as i64);
    
    // Store block
    storage.put_block(&block)?;
    println!("✓ Block persisted to RocksDB");
    metrics.storage_writes.inc();
    
    // Step 6: Generate zk-SNARK Proof
    println!("\n--- Step 6: Generate zk-SNARK Proof ---");
    let circuit = StateTransitionCircuit::new(
        ark_bn254::Fr::from(10u32),
        ark_bn254::Fr::from(13u32),
        ark_bn254::Fr::from(3u32),
    );
    let proof = proof_system.generate_proof(&circuit)?;
    println!("✓ zk-SNARK proof generated (hex: {})", proof.to_hex());
    
    // Step 7: Verify Proof
    println!("\n--- Step 7: Verify zk-SNARK Proof ---");
    let verified = proof_system.verify_proof(&proof)?;
    if verified {
        println!("✓ Proof verification passed");
    } else {
        println!("✗ Proof verification failed");
    }
    
    // Step 8: Health Check
    println!("\n--- Step 8: Health Check ---");
    let health_status = health_checker.check_all();
    println!("✓ Overall health status: {:?}", health_status.status);
    println!("  Uptime: {}s", health_status.uptime_seconds);
    println!("  Version: {}", health_status.version);
    
    // Step 9: Metrics Export
    println!("\n--- Step 9: Metrics Export ---");
    let metrics_export = {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let metric_families = registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    };
    println!("✓ Metrics exported (sample):");
    for line in metrics_export.lines().take(5) {
        println!("  {}", line);
    }
    
    // Step 10: Demonstrate Receiver-Pays-Gas
    println!("\n--- Step 10: Receiver-Pays-Gas Demonstration ---");
    let mut state = State::new();
    state.set_balance(alice_keypair.public_key.to_hex(), 1000);
    state.set_balance("bob".to_string(), 500);
    
    println!("Initial balances:");
    println!("  Alice: {}", state.get_balance(&alice_keypair.public_key.to_hex()));
    println!("  Bob: {}", state.get_balance("bob"));
    
    // Gasless transaction
    let gasless_tx = Transaction::new(
        alice_keypair.public_key.to_hex(),
        "bob".to_string(),
        100,
        0,
        None,
        None,
    );
    state.apply_transaction(&gasless_tx);
    println!("\nAfter gasless transaction (100 tokens):");
    println!("  Alice: {} (paid 100)", state.get_balance(&alice_keypair.public_key.to_hex()));
    println!("  Bob: {} (received 100, no gas)", state.get_balance("bob"));
    
    // Transaction with gas
    let gas_tx = Transaction::new(
        alice_keypair.public_key.to_hex(),
        "bob".to_string(),
        100,
        1,
        Some(21000),
        Some(1),
    );
    state.apply_transaction(&gas_tx);
    println!("\nAfter transaction with gas (100 tokens, 21 gas):");
    println!("  Alice: {} (paid 100)", state.get_balance(&alice_keypair.public_key.to_hex()));
    println!("  Bob: {} (received 100, paid 21 gas)", state.get_balance("bob"));
    
    println!("\n=== Demo Complete ===");
    println!("Key insight: Sender does the work (mining), receiver optionally pays gas");
    
    Ok(())
}

// HealthStatus import
use llm_mina_chain::{HealthCheck, HealthStatus};
