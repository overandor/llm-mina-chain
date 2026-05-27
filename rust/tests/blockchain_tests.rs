#[cfg(test)]
mod blockchain_tests {
    use llm_mina_chain::{Blockchain, Transaction, State, Block};

    #[test]
    fn test_blockchain_creation() {
        let blockchain = Blockchain::new();
        assert_eq!(blockchain.chain.len(), 1); // Genesis block
        assert_eq!(blockchain.get_latest_block().height, 0);
    }

    #[test]
    fn test_transaction_creation() {
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            0,
            Some(21000),
            Some(1),
        );
        
        assert_eq!(tx.sender, "alice");
        assert_eq!(tx.receiver, "bob");
        assert_eq!(tx.amount, 100);
        assert!(!tx.is_gasless());
    }

    #[test]
    fn test_gasless_transaction() {
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            0,
            None,
            None,
        );
        
        assert!(tx.is_gasless());
        assert_eq!(tx.calculate_gas_cost(), 0);
    }

    #[test]
    fn test_state_balance() {
        let mut state = State::new();
        state.set_balance("alice".to_string(), 1000);
        
        assert_eq!(state.get_balance("alice"), 1000);
        assert_eq!(state.get_balance("bob"), 0);
    }

    #[test]
    fn test_state_nonce() {
        let mut state = State::new();
        assert_eq!(state.get_nonce("alice"), 0);
        
        state.increment_nonce("alice");
        assert_eq!(state.get_nonce("alice"), 1);
        
        state.increment_nonce("alice");
        assert_eq!(state.get_nonce("alice"), 2);
    }

    #[test]
    fn test_transaction_application() {
        let mut state = State::new();
        state.set_balance("alice".to_string(), 1000);
        state.set_balance("bob".to_string(), 500);
        
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            0,
            None, // gasless
            None,
        );
        
        let result = state.apply_transaction(&tx);
        assert!(result);
        
        assert_eq!(state.get_balance("alice"), 900);
        assert_eq!(state.get_balance("bob"), 600);
        assert_eq!(state.get_nonce("alice"), 1);
    }

    #[test]
    fn test_transaction_with_gas() {
        let mut state = State::new();
        state.set_balance("alice".to_string(), 1000);
        state.set_balance("bob".to_string(), 500);
        
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            0,
            Some(21),
            Some(1),
        );
        
        let result = state.apply_transaction(&tx);
        assert!(result);
        
        // Sender pays amount, receiver pays gas
        assert_eq!(state.get_balance("alice"), 900);
        assert_eq!(state.get_balance("bob"), 579); // 500 + 100 - 21 (gas)
    }

    #[test]
    fn test_insufficient_balance() {
        let mut state = State::new();
        state.set_balance("alice".to_string(), 50);
        
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            0,
            None,
            None,
        );
        
        let result = state.apply_transaction(&tx);
        assert!(!result);
    }

    #[test]
    fn test_invalid_nonce() {
        let mut state = State::new();
        state.set_balance("alice".to_string(), 1000);
        state.increment_nonce("alice"); // nonce is now 1
        
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            0, // nonce is 0, but state expects 1
            None,
            None,
        );
        
        let result = state.apply_transaction(&tx);
        assert!(!result);
    }

    #[test]
    fn test_block_creation() {
        let mut blockchain = Blockchain::new();
        
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            0,
            None,
            None,
        );
        
        let block = blockchain.create_block(vec![tx]);
        assert!(block.is_some());
        
        let block = block.unwrap();
        assert_eq!(block.height, 1);
        assert_eq!(block.transactions.len(), 1);
    }

    #[test]
    fn test_block_hash() {
        let block = Block::new(
            1,
            vec![],
            "prev_hash".to_string(),
            "state_hash".to_string(),
        );
        
        let hash1 = block.compute_hash();
        let hash2 = block.compute_hash();
        
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_transaction_pool() {
        let mut blockchain = Blockchain::new();
        
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            0,
            None,
            None,
        );
        
        assert!(blockchain.add_transaction(tx));
        assert_eq!(blockchain.transaction_pool.len(), 1);
    }

    #[test]
    fn test_gas_price() {
        let mut blockchain = Blockchain::new();
        
        blockchain.set_gas_price(5);
        assert_eq!(blockchain.get_gas_price(), 5);
        
        blockchain.set_gas_price(0); // Should be clamped to min_gas_price
        assert_eq!(blockchain.get_gas_price(), 0);
    }
}
