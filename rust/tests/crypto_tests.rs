#[cfg(test)]
mod crypto_tests {
    use llm_mina_chain::{KeyPair, PublicKey, PrivateKey, DigitalSignature, CryptoError};

    #[test]
    fn test_keypair_generation() {
        let keypair = KeyPair::generate();
        let derived_public = keypair.private_key.public_key();
        assert_eq!(keypair.public_key, derived_public);
    }

    #[test]
    fn test_sign_and_verify() {
        let keypair = KeyPair::generate();
        let message = b"Hello, world!";
        
        let signature = keypair.sign(message);
        let result = keypair.verify(message, &signature);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_signature() {
        let keypair = KeyPair::generate();
        let message = b"Hello, world!";
        
        let signature = keypair.sign(message);
        let wrong_message = b"Wrong message";
        let result = keypair.verify(wrong_message, &signature);
        assert!(result.is_err());
    }

    #[test]
    fn test_public_key_hex() {
        let keypair = KeyPair::generate();
        let hex = keypair.public_key.to_hex();
        let recovered = PublicKey::from_hex(&hex).unwrap();
        assert_eq!(keypair.public_key, recovered);
    }

    #[test]
    fn test_signature_hex() {
        let keypair = KeyPair::generate();
        let message = b"Test message";
        let signature = keypair.sign(message);
        
        let hex = signature.to_hex();
        let recovered = DigitalSignature::from_hex(&hex).unwrap();
        assert_eq!(signature, recovered);
    }

    #[test]
    fn test_invalid_hex() {
        let result = PublicKey::from_hex("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_wrong_length_hex() {
        let result = PublicKey::from_hex("abcd");
        assert!(matches!(result, Err(CryptoError::InvalidLength)));
    }
}
