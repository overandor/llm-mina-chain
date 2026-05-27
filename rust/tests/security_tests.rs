#[cfg(test)]
mod security_tests {
    use llm_mina_chain::{InputValidator, SecurityConfig, ValidationResult, Transaction, Block};
    use prometheus::Registry;

    #[test]
    fn test_security_config_default() {
        let config = SecurityConfig::default();
        assert_eq!(config.max_transaction_amount, 1_000_000_000);
        assert_eq!(config.max_gas_limit, 10_000_000);
        assert!(config.require_signatures);
    }

    #[test]
    fn test_validation_result() {
        let mut result = ValidationResult::new();
        assert!(result.is_valid());
        
        result.add_error("Test error".to_string());
        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);
        
        result.add_warning("Test warning".to_string());
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn test_valid_transaction() {
        let registry = Registry::new();
        let config = SecurityConfig::default();
        let validator = InputValidator::new(config, &registry);
        
        let keypair = llm_mina_chain::KeyPair::generate();
        let mut tx = Transaction::new(
            keypair.public_key.to_hex(),
            "bob".to_string(),
            100,
            0,
            Some(21000),
            Some(1),
        );
        tx.sign(&keypair);
        
        let result = validator.validate_transaction(&tx);
        assert!(result.is_valid());
    }

    #[test]
    fn test_zero_amount_transaction() {
        let registry = Registry::new();
        let config = SecurityConfig::default();
        let validator = InputValidator::new(config, &registry);
        
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            0,
            0,
            None,
            None,
        );
        
        let result = validator.validate_transaction(&tx);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.contains("zero")));
    }

    #[test]
    fn test_excessive_amount() {
        let registry = Registry::new();
        let config = SecurityConfig::default();
        let validator = InputValidator::new(config, &registry);
        
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            2_000_000_000,
            0,
            None,
            None,
        );
        
        let result = validator.validate_transaction(&tx);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.contains("exceeds maximum")));
    }

    #[test]
    fn test_same_sender_receiver() {
        let registry = Registry::new();
        let config = SecurityConfig::default();
        let validator = InputValidator::new(config, &registry);
        
        let tx = Transaction::new(
            "alice".to_string(),
            "alice".to_string(),
            100,
            0,
            None,
            None,
        );
        
        let result = validator.validate_transaction(&tx);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.contains("cannot be the same")));
    }

    #[test]
    fn test_low_gas_limit() {
        let registry = Registry::new();
        let config = SecurityConfig::default();
        let validator = InputValidator::new(config, &registry);
        
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            0,
            Some(10000), // Below minimum
            Some(1),
        );
        
        let result = validator.validate_transaction(&tx);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.contains("too low")));
    }

    #[test]
    fn test_high_gas_price() {
        let registry = Registry::new();
        let config = SecurityConfig::default();
        let validator = InputValidator::new(config, &registry);
        
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            0,
            Some(21000),
            Some(2000), // Above maximum
        );
        
        let result = validator.validate_transaction(&tx);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.contains("exceeds maximum")));
    }

    #[test]
    fn test_missing_signature() {
        let registry = Registry::new();
        let config = SecurityConfig {
            require_signatures: true,
            ..Default::default()
        };
        let validator = InputValidator::new(config, &registry);
        
        let tx = Transaction::new(
            "alice".to_string(),
            "bob".to_string(),
            100,
            0,
            None,
            None,
        );
        
        let result = validator.validate_transaction(&tx);
        assert!(!result.is_valid());
        assert!(result.errors.iter().any(|e| e.contains("Signature required")));
    }

    #[test]
    fn test_valid_block() {
        let registry = Registry::new();
        let config = SecurityConfig::default();
        let validator = InputValidator::new(config, &registry);
        
        let block = Block::new(
            1,
            vec![],
            "prev_hash".to_string(),
            "state_hash".to_string(),
        );
        
        let result = validator.validate_block(&block);
        assert!(result.is_valid());
    }

    #[test]
    fn test_block_with_invalid_transaction() {
        let registry = Registry::new();
        let config = SecurityConfig::default();
        let validator = InputValidator::new(config, &registry);
        
        let tx = Transaction::new(
            "alice".to_string(),
            "alice".to_string(), // Invalid: same sender/receiver
            100,
            0,
            None,
            None,
        );
        
        let block = Block::new(
            1,
            vec![tx],
            "prev_hash".to_string(),
            "state_hash".to_string(),
        );
        
        let result = validator.validate_block(&block);
        assert!(!result.is_valid());
    }
}
