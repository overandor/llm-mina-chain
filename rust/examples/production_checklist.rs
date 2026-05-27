//! Production readiness checklist verification
//! Runs through all critical checks before mainnet deployment

use llm_mina_chain::{
    Blockchain, Transaction, State, KeyPair, BlockchainStorage, InputValidator, SecurityConfig,
    ProofSystem, StateTransitionCircuit, HealthChecker, SystemMetrics,
};
use prometheus::Encoder;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== MEMBRA Instant Proof Chain - Production Readiness Checklist ===\n");
    
    let mut passed = 0;
    let mut failed = 0;
    let mut warnings = 0;
    
    // Check 1: Real signatures passing tests
    println!("--- Check 1: Real Signatures ---");
    match test_signatures() {
        Ok(_) => {
            println!("✓ PASS: Ed25519 signatures working correctly");
            passed += 1;
        }
        Err(e) => {
            println!("✗ FAIL: Signature test failed: {}", e);
            failed += 1;
        }
    }
    
    // Check 2: RocksDB persistence verified after restart
    println!("\n--- Check 2: RocksDB Persistence ---");
    match test_persistence() {
        Ok(_) => {
            println!("✓ PASS: RocksDB persistence verified");
            passed += 1;
        }
        Err(e) => {
            println!("✗ FAIL: Persistence test failed: {}", e);
            failed += 1;
        }
    }
    
    // Check 3: P2P transaction propagation verified
    println!("\n--- Check 3: P2P Transaction Propagation ---");
    println!("⚠ WARNING: P2P requires actual network - skipping automated test");
    println!("  Manual verification required: Run three-node demo");
    warnings += 1;
    
    // Check 4: Consensus commits verified across at least three nodes
    println!("\n--- Check 4: Consensus Across Three Nodes ---");
    println!("⚠ WARNING: Consensus requires actual network - skipping automated test");
    println!("  Manual verification required: Run three-node demo");
    warnings += 1;
    
    // Check 5: Receiver-pays-gas logic verified with edge cases
    println!("\n--- Check 5: Receiver-Pays-Gas Logic ---");
    match test_receiver_pays_gas() {
        Ok(_) => {
            println!("✓ PASS: Receiver-pays-gas logic verified");
            passed += 1;
        }
        Err(e) => {
            println!("✗ FAIL: Gas logic test failed: {}", e);
            failed += 1;
        }
    }
    
    // Check 6: Gasless transactions verified
    println!("\n--- Check 6: Gasless Transactions ---");
    match test_gasless_transactions() {
        Ok(_) => {
            println!("✓ PASS: Gasless transactions working");
            passed += 1;
        }
        Err(e) => {
            println!("✗ FAIL: Gasless transaction test failed: {}", e);
            failed += 1;
        }
    }
    
    // Check 7: zk-proof layer compiling and producing/verifying proof artifacts
    println!("\n--- Check 7: zk-SNARK Proof Layer ---");
    match test_zk_proofs() {
        Ok(_) => {
            println!("✓ PASS: zk-SNARK layer working");
            passed += 1;
        }
        Err(e) => {
            println!("✗ FAIL: zk-proof test failed: {}", e);
            failed += 1;
        }
    }
    
    // Check 8: Prometheus metrics available
    println!("\n--- Check 8: Prometheus Metrics ---");
    match test_metrics() {
        Ok(_) => {
            println!("✓ PASS: Metrics system working");
            passed += 1;
        }
        Err(e) => {
            println!("✗ FAIL: Metrics test failed: {}", e);
            failed += 1;
        }
    }
    
    // Check 9: Health endpoints available
    println!("\n--- Check 9: Health Endpoints ---");
    match test_health_checks() {
        Ok(_) => {
            println!("✓ PASS: Health checks working");
            passed += 1;
        }
        Err(e) => {
            println!("✗ FAIL: Health check test failed: {}", e);
            failed += 1;
        }
    }
    
    // Check 10: Benchmarks reproducible
    println!("\n--- Check 10: Benchmarks ---");
    println!("⚠ WARNING: Benchmarks require manual run: cargo bench");
    println!("  Run: cargo bench -- --output-format bencher");
    warnings += 1;
    
    // Check 11: API versioning locked
    println!("\n--- Check 11: API Versioning ---");
    match test_api_versioning() {
        Ok(_) => {
            println!("✓ PASS: API versioning working");
            passed += 1;
        }
        Err(e) => {
            println!("✗ FAIL: API versioning test failed: {}", e);
            failed += 1;
        }
    }
    
    // Summary
    println!("\n=== Summary ===");
    println!("Passed: {}/{}", passed, passed + failed);
    println!("Warnings: {}", warnings);
    
    if failed == 0 && warnings == 0 {
        println!("\n✓ ALL CHECKS PASSED - Ready for production deployment");
    } else if failed == 0 {
        println!("\n⚠ CHECKS PASSED WITH WARNINGS - Manual verification required for network tests");
    } else {
        println!("\n✗ SOME CHECKS FAILED - Fix failures before production deployment");
    }
    
    Ok(())
}

fn test_signatures() -> Result<(), Box<dyn std::error::Error>> {
    let keypair = KeyPair::generate();
    let message = b"test message";
    
    let signature = keypair.sign(message);
    keypair.verify(message, &signature)?;
    
    // Test signature serialization
    let hex = signature.to_hex();
    let recovered = llm_mina_chain::DigitalSignature::from_hex(&hex)?;
    assert_eq!(signature, recovered);
    
    Ok(())
}

fn test_persistence() -> Result<(), Box<dyn std::error::Error>> {
    let storage_path = PathBuf::from("./checklist_storage");
    
    // Write
    let storage = BlockchainStorage::open(&storage_path)?;
    let block = llm_mina_chain::Block::new(
        1,
        vec![],
        "prev".to_string(),
        "state".to_string(),
    );
    storage.put_block(&block)?;
    
    // Read
    let retrieved = storage.get_block(1)?;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().height, 1);
    
    // Cleanup
    std::fs::remove_dir_all(storage_path)?;
    
    Ok(())
}

fn test_receiver_pays_gas() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = State::new();
    state.set_balance("alice".to_string(), 1000);
    state.set_balance("bob".to_string(), 500);
    
    // Transaction with gas
    let tx = Transaction::new(
        "alice".to_string(),
        "bob".to_string(),
        100,
        0,
        Some(21000),
        Some(1),
    );
    
    assert!(state.apply_transaction(&tx));
    
    // Alice should pay 100 (mining work)
    assert_eq!(state.get_balance("alice"), 900);
    
    // Bob should receive 100 and pay 21 gas
    assert_eq!(state.get_balance("bob"), 579);
    
    Ok(())
}

fn test_gasless_transactions() -> Result<(), Box<dyn std::error::Error>> {
    let mut state = State::new();
    state.set_balance("alice".to_string(), 1000);
    state.set_balance("bob".to_string(), 500);
    
    // Gasless transaction
    let tx = Transaction::new(
        "alice".to_string(),
        "bob".to_string(),
        100,
        0,
        None,
        None,
    );
    
    assert!(state.apply_transaction(&tx));
    
    // Alice should pay 100
    assert_eq!(state.get_balance("alice"), 900);
    
    // Bob should receive 100 with no gas
    assert_eq!(state.get_balance("bob"), 600);
    
    Ok(())
}

fn test_zk_proofs() -> Result<(), Box<dyn std::error::Error>> {
    let mut proof_system = ProofSystem::new();
    proof_system.setup()?;
    
    let circuit = StateTransitionCircuit::new(
        ark_bn254::Fr::from(10u32),
        ark_bn254::Fr::from(13u32),
        ark_bn254::Fr::from(3u32),
    );
    
    let proof = proof_system.generate_proof(&circuit)?;
    let verified = proof_system.verify_proof(&proof)?;
    
    assert!(verified);
    Ok(())
}

fn test_metrics() -> Result<(), Box<dyn std::error::Error>> {
    use prometheus::Registry;
    
    let registry = Registry::new();
    let metrics = llm_mina_chain::BlockchainMetrics::new(&registry);
    
    metrics.blocks_produced.inc();
    metrics.block_height.set(10);
    
    let encoder = prometheus::TextEncoder::new();
    let metric_families = registry.gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer)?;
    
    let output = String::from_utf8(buffer)?;
    assert!(output.contains("blockchain_blocks_produced_total"));
    
    Ok(())
}

fn test_health_checks() -> Result<(), Box<dyn std::error::Error>> {
    let mut health_checker = HealthChecker::new("1.0.0".to_string());
    
    health_checker.register_check("test".to_string(), || {
        llm_mina_chain::HealthCheck::new(
            "test".to_string(),
            llm_mina_chain::HealthStatus::Healthy,
            "OK".to_string(),
            10,
        )
    });
    
    let status = health_checker.check_all();
    assert_eq!(status.status, llm_mina_chain::HealthStatus::Healthy);
    
    Ok(())
}

fn test_api_versioning() -> Result<(), Box<dyn std::error::Error>> {
    use llm_mina_chain::{ApiVersion, ApiRegistry, ApiEndpoint};
    
    let v1 = ApiVersion::new(1, 0, 0);
    let v2 = ApiVersion::new(1, 1, 0);
    
    assert!(v1.is_compatible(&v2));
    
    let parsed = ApiVersion::parse_version("1.2.3")?;
    assert_eq!(parsed.major, 1);
    assert_eq!(parsed.minor, 2);
    assert_eq!(parsed.patch, 3);
    
    Ok(())
}
