//! Replay Validation Tests
//!
//! These tests enforce deterministic execution:
//! - Same inputs must always produce same state root
//! - Same block must always produce same hash
//! - Receipts must be reproducible from raw data

use llm_mina_chain::{
    Blockchain, Transaction, State, Block, KeyPair,
};

/// Deterministic: same transaction sequence produces same state root
#[test]
fn test_replay_deterministic_state() {
    let mut state1 = State::new();
    let mut state2 = State::new();

    let txs = vec![
        Transaction::new_with_timestamp("alice".into(), "bob".into(), 100, 0, None, None, 1700000000),
        Transaction::new_with_timestamp("bob".into(), "charlie".into(), 50, 0, None, None, 1700000001),
        Transaction::new_with_timestamp("charlie".into(), "alice".into(), 25, 0, None, None, 1700000002),
    ];

    for tx in &txs {
        state1.apply_transaction(tx);
    }

    for tx in &txs {
        state2.apply_transaction(tx);
    }

    assert_eq!(state1.balances, state2.balances);
    assert_eq!(state1.nonces, state2.nonces);
}

/// Deterministic: same block params produce same hash
#[test]
fn test_replay_deterministic_block_hash() {
    let b1 = Block::new(
        1,
        vec![Transaction::new_with_timestamp("a".into(), "b".into(), 10, 0, None, None, 1700000000)],
        "prev".into(),
        "state".into(),
    );

    let b2 = Block::new(
        1,
        vec![Transaction::new_with_timestamp("a".into(), "b".into(), 10, 0, None, None, 1700000000)],
        "prev".into(),
        "state".into(),
    );

    assert_eq!(b1.block_hash, b2.block_hash);
    assert_eq!(b1.compute_hash(), b2.compute_hash());
}

/// Receipt reproduction: given raw inputs, final state must be identical.
/// Note: block hashes differ because timestamps are non-deterministic in production.
/// Replay correctness is verified by state equality, not block hash equality.
#[test]
fn test_replay_receipt_reproducible() {
    let mut blockchain = Blockchain::new();

    let tx = Transaction::new_with_timestamp("alice".into(), "bob".into(), 100, 0, None, None, 1700000000);

    assert!(blockchain.add_transaction(tx.clone()));
    let _block = blockchain.create_block(vec![tx.clone()]).unwrap();

    // Reproduce: same tx on fresh chain (genesis gives same initial state)
    let mut replay = Blockchain::new();
    assert!(replay.add_transaction(tx.clone()));
    let _replay_block = replay.create_block(vec![tx]).unwrap();

    // State must match exactly; block hashes may differ due to timestamps
    assert_eq!(blockchain.state.balances, replay.state.balances);
    assert_eq!(blockchain.state.nonces, replay.state.nonces);
}

/// Signature verification must be deterministic
#[test]
fn test_replay_signature_deterministic() {
    let keypair = KeyPair::generate();
    let message = b"deterministic test message";

    let sig1 = keypair.sign(message);
    let sig2 = keypair.sign(message);

    assert_eq!(sig1, sig2);
    assert!(keypair.public_key.verify(message, &sig1).is_ok());
    assert!(keypair.public_key.verify(message, &sig2).is_ok());
}

/// Transaction ID must be deterministic from inputs
#[test]
fn test_replay_tx_id_deterministic() {
    let tx1 = Transaction::new_with_timestamp("alice".into(), "bob".into(), 100, 0, None, None, 1700000000);
    let tx2 = Transaction::new_with_timestamp("alice".into(), "bob".into(), 100, 0, None, None, 1700000000);

    assert_eq!(tx1.tx_id, tx2.tx_id);
    assert_eq!(tx1.sender, tx2.sender);
    assert_eq!(tx1.receiver, tx2.receiver);
}

/// Empty block must be deterministic
#[test]
fn test_replay_empty_block() {
    let b1 = Block::new(1, vec![], "prev".into(), "state".into());
    let b2 = Block::new(1, vec![], "prev".into(), "state".into());

    assert_eq!(b1.block_hash, b2.block_hash);
}

/// Chain replay: full chain from genesis produces same final state
#[test]
fn test_replay_full_chain() {
    let mut blockchain = Blockchain::new();
    // Genesis gives alice=1000, bob=1000

    let txs = vec![
        Transaction::new_with_timestamp("alice".into(), "bob".into(), 100, 0, None, None, 1700000000),
        Transaction::new_with_timestamp("bob".into(), "alice".into(), 50, 0, None, None, 1700000001),
    ];

    for tx in &txs {
        blockchain.add_transaction(tx.clone());
    }

    let _ = blockchain.create_block(txs.clone());

    let final_balances = blockchain.state.balances.clone();

    // Replay on fresh state: apply genesis + same txs
    let mut replay_state = State::new();
    replay_state.set_balance("genesis".into(), 1_000_000);
    replay_state.set_balance("alice".into(), 1_000);
    replay_state.set_balance("bob".into(), 1_000);

    for tx in &txs {
        replay_state.apply_transaction(tx);
    }

    // Compare only the accounts we care about (not genesis)
    assert_eq!(final_balances.get("alice"), replay_state.balances.get("alice"));
    assert_eq!(final_balances.get("bob"), replay_state.balances.get("bob"));
}

/// Blockchain::replay must reproduce state from block history
#[test]
fn test_blockchain_replay_method() {
    let mut blockchain = Blockchain::new();
    // Use genesis balances: alice=1000, bob=1000 (set by create_genesis_block)

    let tx = Transaction::new_with_timestamp("alice".into(), "bob".into(), 100, 0, None, None, 1700000000);
    blockchain.add_transaction(tx.clone());
    let _ = blockchain.create_block(vec![tx]);

    let replayed = blockchain.replay(Some(&blockchain.state)).expect("replay should succeed");
    assert_eq!(replayed.balances, blockchain.state.balances);
    assert_eq!(replayed.nonces, blockchain.state.nonces);
}

/// Blockchain::verify_chain must pass on a valid chain
#[test]
fn test_blockchain_verify_chain() {
    let mut blockchain = Blockchain::new();
    let tx = Transaction::new_with_timestamp("alice".into(), "bob".into(), 50, 0, None, None, 1700000000);
    blockchain.add_transaction(tx.clone());
    let _ = blockchain.create_block(vec![tx]);

    blockchain.verify_chain().expect("verify_chain should pass on valid chain");
}

/// verify_chain must detect tampered block hash
#[test]
fn test_blockchain_verify_chain_detects_tampering() {
    let mut blockchain = Blockchain::new();
    let tx = Transaction::new_with_timestamp("alice".into(), "bob".into(), 50, 0, None, None, 1700000000);
    blockchain.add_transaction(tx.clone());
    let _ = blockchain.create_block(vec![tx]);

    // Tamper with a block hash
    blockchain.chain[1].block_hash = "evil_hash".to_string();

    let result = blockchain.verify_chain();
    assert!(result.is_err(), "verify_chain should fail on tampered hash");
    let err = result.unwrap_err();
    assert!(err.contains("hash mismatch"), "error should mention hash mismatch: {}", err);
}
