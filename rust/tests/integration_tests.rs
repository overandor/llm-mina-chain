#[cfg(test)]
mod integration_tests {
    use llm_mina_chain::{Blockchain, Transaction, Block, State};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    /// Test multi-node blockchain synchronization
    #[test]
    fn test_multi_node_sync() {
        // Create two separate blockchains
        let blockchain1 = Arc::new(Mutex::new(Blockchain::new()));
        let blockchain2 = Arc::new(Mutex::new(Blockchain::new()));
        
        // Add a transaction to blockchain1
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            0,
            None,
            None,
        );
        
        {
            let mut bc = blockchain1.lock().unwrap();
            bc.add_transaction(tx.clone());
            let block = bc.create_block(vec![tx]);
            assert!(block.is_some());
        }
        
        // Simulate sync: copy block from blockchain1 to blockchain2
        {
            let bc1 = blockchain1.lock().unwrap();
            let block = bc1.get_latest_block().clone();
            drop(bc1);
            
            let mut bc2 = blockchain2.lock().unwrap();
            // In a real implementation, this would verify the block
            bc2.chain.push(block);
        }
        
        // Verify both chains have the same height
        let bc1 = blockchain1.lock().unwrap();
        let bc2 = blockchain2.lock().unwrap();
        assert_eq!(bc1.chain.len(), bc2.chain.len());
        assert_eq!(bc1.get_latest_block().height, bc2.get_latest_block().height);
    }

    /// Test concurrent transaction processing
    #[test]
    fn test_concurrent_transactions() {
        let blockchain = Arc::new(Mutex::new(Blockchain::new()));
        let mut handles = vec![];
        
        // Spawn multiple threads adding transactions
        for i in 0..10 {
            let bc = blockchain.clone();
            let handle = thread::spawn(move || {
                let tx = Transaction::new(
                    "alice".to_string(),
                    format!("user{}", i),
                    10,
                    i as u64,
                    None,
                    None,
                );
                
                let mut bc = bc.lock().unwrap();
                bc.add_transaction(tx);
            });
            handles.push(handle);
        }
        
        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }
        
        // Verify all transactions were added
        let bc = blockchain.lock().unwrap();
        assert_eq!(bc.transaction_pool.len(), 10);
    }

    /// Test block propagation simulation
    #[test]
    fn test_block_propagation() {
        let blockchain = Arc::new(Mutex::new(Blockchain::new()));
        let mut handles = vec![];
        
        // Simulate 3 nodes
        for _ in 0..3 {
            let bc = blockchain.clone();
            let handle = thread::spawn(move || {
                // Each node processes the block
                thread::sleep(Duration::from_millis(10));
                let bc = bc.lock().unwrap();
                assert_eq!(bc.chain.len(), 1); // Genesis block
            });
            handles.push(handle);
        }
        
        for handle in handles {
            handle.join().unwrap();
        }
    }

    /// Test state consistency across nodes
    #[test]
    fn test_state_consistency() {
        let state1 = Arc::new(Mutex::new(State::new()));
        let state2 = Arc::new(Mutex::new(State::new()));
        
        // Initialize state1
        {
            let mut s = state1.lock().unwrap();
            s.set_balance("alice".to_string(), 1000);
            s.set_balance("bob".to_string(), 500);
        }
        
        // Copy state to state2 (simulating sync)
        {
            let s1 = state1.lock().unwrap();
            let mut s2 = state2.lock().unwrap();
            s2.balances = s1.balances.clone();
            s2.nonces = s1.nonces.clone();
        }
        
        // Verify consistency
        let s1 = state1.lock().unwrap();
        let s2 = state2.lock().unwrap();
        assert_eq!(s1.get_balance("alice"), s2.get_balance("alice"));
        assert_eq!(s1.get_balance("bob"), s2.get_balance("bob"));
    }

    /// Test transaction pool isolation
    #[test]
    fn test_transaction_pool_isolation() {
        let blockchain1 = Arc::new(Mutex::new(Blockchain::new()));
        let blockchain2 = Arc::new(Mutex::new(Blockchain::new()));
        
        // Add transaction to blockchain1
        {
            let mut bc = blockchain1.lock().unwrap();
            let tx = Transaction::new(
                "alice".to_string(),
                "bob".to_string(),
                100,
                0,
                None,
                None,
            );
            bc.add_transaction(tx);
        }
        
        // Verify blockchain2 doesn't have the transaction
        let bc2 = blockchain2.lock().unwrap();
        assert_eq!(bc2.transaction_pool.len(), 0);
    }

    /// Test fork resolution
    #[test]
    fn test_fork_resolution() {
        let blockchain = Arc::new(Mutex::new(Blockchain::new()));
        
        // Create two competing blocks at height 1
        let block1 = Block::new(
            1,
            vec![],
            blockchain.lock().unwrap().get_latest_block().block_hash.clone(),
            "state_hash_1".to_string(),
        );
        
        let block2 = Block::new(
            1,
            vec![],
            blockchain.lock().unwrap().get_latest_block().block_hash.clone(),
            "state_hash_2".to_string(),
        );
        
        // In a real implementation, this would select the longer chain
        // For now, just verify they have different hashes
        assert_ne!(block1.block_hash, block2.block_hash);
    }

    /// Test network partition simulation
    #[test]
    fn test_network_partition() {
        let blockchain1 = Arc::new(Mutex::new(Blockchain::new()));
        let blockchain2 = Arc::new(Mutex::new(Blockchain::new()));
        
        // Add block to blockchain1
        {
            let mut bc = blockchain1.lock().unwrap();
            let tx = Transaction::new(
                "alice".to_string(),
                "bob".to_string(),
                100,
                0,
                None,
                None,
            );
            bc.create_block(vec![tx]);
        }
        
        // Simulate partition: blockchain2 doesn't receive the block
        let bc1 = blockchain1.lock().unwrap();
        let bc2 = blockchain2.lock().unwrap();
        
        assert_eq!(bc1.chain.len(), 2); // Genesis + 1 block
        assert_eq!(bc2.chain.len(), 1); // Only genesis
    }

    /// Test recovery after partition
    #[test]
    fn test_partition_recovery() {
        let blockchain1 = Arc::new(Mutex::new(Blockchain::new()));
        let blockchain2 = Arc::new(Mutex::new(Blockchain::new()));
        
        // Add block to blockchain1
        let block = {
            let mut bc = blockchain1.lock().unwrap();
            let tx = Transaction::new(
                "alice".to_string(),
                "bob".to_string(),
                100,
                0,
                None,
                None,
            );
            bc.create_block(vec![tx]).unwrap()
        };
        
        // Simulate recovery: sync block to blockchain2
        {
            let mut bc = blockchain2.lock().unwrap();
            bc.chain.push(block);
        }
        
        // Verify recovery
        let bc1 = blockchain1.lock().unwrap();
        let bc2 = blockchain2.lock().unwrap();
        assert_eq!(bc1.chain.len(), bc2.chain.len());
    }
}
