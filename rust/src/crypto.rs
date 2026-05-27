//! Cryptographic primitives: Ed25519 signatures and key management
//! Production-grade implementation using ed25519-dalek 2.0

use ed25519_dalek::{Signer, Verifier, SigningKey, VerifyingKey, Signature};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Public key wrapper (32 bytes)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublicKey([u8; 32]);

/// Private key wrapper (32 bytes)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateKey([u8; 32]);

/// Key pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    pub public_key: PublicKey,
    pub private_key: PrivateKey,
}

/// Digital signature wrapper (64 bytes)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DigitalSignature([u8; 64]);

impl Serialize for DigitalSignature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for DigitalSignature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = DigitalSignature;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a 64-byte signature")
            }
            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.len() != 64 {
                    return Err(serde::de::Error::invalid_length(v.len(), &self));
                }
                let mut arr = [0u8; 64];
                arr.copy_from_slice(v);
                Ok(DigitalSignature(arr))
            }
        }
        deserializer.deserialize_bytes(Visitor)
    }
}

impl PublicKey {
    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        PublicKey(bytes)
    }
    
    /// Convert to bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    
    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
    
    /// Create from hex string
    pub fn from_hex(hex_str: &str) -> Result<Self, CryptoError> {
        let bytes = hex::decode(hex_str)
            .map_err(|_| CryptoError::InvalidHex)?;
        
        if bytes.len() != 32 {
            return Err(CryptoError::InvalidLength);
        }
        
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(PublicKey(arr))
    }
    
    /// Verify a signature using real Ed25519 verification
    #[tracing::instrument(skip(self, message, signature), fields(pk_hash = %self.to_hex()[..8]))]
    pub fn verify(&self, message: &[u8], signature: &DigitalSignature) -> Result<(), CryptoError> {
        let verifying_key = VerifyingKey::from_bytes(self.as_bytes())
            .map_err(|_| CryptoError::InvalidPublicKey)?;
        let sig = Signature::from_bytes(signature.as_bytes());
        verifying_key.verify(message, &sig)
            .map_err(|_| CryptoError::InvalidSignature)
    }
}

impl PrivateKey {
    /// Generate new private key using cryptographically secure RNG
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut bytes);
        PrivateKey(bytes)
    }
    
    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        PrivateKey(bytes)
    }
    
    /// Convert to bytes
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    
    /// Derive public key using Ed25519 key derivation
    pub fn public_key(&self) -> PublicKey {
        let signing_key = SigningKey::from_bytes(self.as_bytes());
        let verifying_key = signing_key.verifying_key();
        PublicKey(verifying_key.to_bytes())
    }
    
    /// Sign a message using real Ed25519 signing
    #[tracing::instrument(skip(self, message))]
    pub fn sign(&self, message: &[u8]) -> DigitalSignature {
        let signing_key = SigningKey::from_bytes(self.as_bytes());
        let signature = signing_key.sign(message);
        DigitalSignature(signature.to_bytes())
    }
}

impl KeyPair {
    /// Generate new key pair using real Ed25519
    #[tracing::instrument]
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut rand::rngs::OsRng);
        let verifying_key = signing_key.verifying_key();
        KeyPair {
            public_key: PublicKey(verifying_key.to_bytes()),
            private_key: PrivateKey(signing_key.to_bytes()),
        }
    }
    
    /// Sign a message
    #[tracing::instrument(skip(self, message))]
    pub fn sign(&self, message: &[u8]) -> DigitalSignature {
        self.private_key.sign(message)
    }
    
    /// Verify a signature
    #[tracing::instrument(skip(self, message, signature))]
    pub fn verify(&self, message: &[u8], signature: &DigitalSignature) -> Result<(), CryptoError> {
        self.public_key.verify(message, signature)
    }
}

impl DigitalSignature {
    /// Create from bytes
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        DigitalSignature(bytes)
    }
    
    /// Convert to bytes
    pub fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }
    
    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
    
    /// Create from hex string
    pub fn from_hex(hex_str: &str) -> Result<Self, CryptoError> {
        let bytes = hex::decode(hex_str)
            .map_err(|_| CryptoError::InvalidHex)?;
        
        if bytes.len() != 64 {
            return Err(CryptoError::InvalidLength);
        }
        
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        Ok(DigitalSignature(arr))
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl fmt::Display for DigitalSignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Cryptographic errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CryptoError {
    InvalidHex,
    InvalidLength,
    InvalidPublicKey,
    InvalidSignature,
    SigningError,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoError::InvalidHex => write!(f, "Invalid hex string"),
            CryptoError::InvalidLength => write!(f, "Invalid length"),
            CryptoError::InvalidPublicKey => write!(f, "Invalid public key"),
            CryptoError::InvalidSignature => write!(f, "Invalid signature"),
            CryptoError::SigningError => write!(f, "Signing error"),
        }
    }
}

impl std::error::Error for CryptoError {}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_key_generation() {
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
}
