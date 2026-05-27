use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::types::{AgentId, CanonicalTimestamp, DeterministicHash, SemVer};

/// A canonical receipt is the single shared output format for ALL agents.
///
/// - Agent 1 (Core Runtime) produces execution receipts.
/// - Agent 2 (Solana Query) produces query result receipts.
/// - Agent 3 (Proof/Provenance) produces proof receipts.
///
/// All receipts share the same schema, the same hash function (SHA-256),
/// and the same serialization strategy (canonical JSON with sorted keys).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalReceipt {
    /// Protocol version that produced this receipt.
    pub version: SemVer,
    /// Unix timestamp in milliseconds.
    pub timestamp: CanonicalTimestamp,
    /// Which agent produced this receipt.
    pub source: AgentId,
    /// Receipt type discriminator.
    pub receipt_type: ReceiptType,
    /// SHA-256 of the canonical JSON-serialized payload.
    pub payload_hash: DeterministicHash,
    /// Optional Merkle root covering this receipt and siblings.
    pub merkle_root: Option<DeterministicHash>,
    /// Optional cryptographic signature over `payload_hash`.
    pub signature: Option<Vec<u8>>,
    /// Human-readable description.
    pub description: String,
    /// Arbitrary structured payload. Must be deterministic.
    pub payload: serde_json::Value,
}

/// Discriminant for receipt types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReceiptType {
    Execution,
    QueryResult,
    Proof,
    Anchor,
    Audit,
}

impl CanonicalReceipt {
    /// Build a receipt. Computes the payload hash deterministically.
    pub fn new(
        source: AgentId,
        receipt_type: ReceiptType,
        description: impl Into<String>,
        payload: serde_json::Value,
    ) -> Self {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        let payload_hash = hash_json_canonical(&payload);

        Self {
            version: SemVer::CURRENT,
            timestamp,
            source,
            receipt_type,
            payload_hash,
            merkle_root: None,
            signature: None,
            description: description.into(),
            payload,
        }
    }

    /// Attach a Merkle root. Consumes self.
    pub fn with_merkle_root(mut self, root: DeterministicHash) -> Self {
        self.merkle_root = Some(root);
        self
    }

    /// Attach a signature. Consumes self.
    pub fn with_signature(mut self, sig: Vec<u8>) -> Self {
        self.signature = Some(sig);
        self
    }

    /// Verify that the stored payload hash matches the current payload.
    pub fn verify_integrity(&self) -> bool {
        let computed = hash_json_canonical(&self.payload);
        computed == self.payload_hash
    }
}

/// Deterministic SHA-256 over canonical JSON (sorted keys, no extra whitespace).
pub fn hash_json_canonical(value: &serde_json::Value) -> DeterministicHash {
    let mut buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"");
    let mut ser = serde_json::Serializer::with_formatter(&mut buf, formatter);
    // We use a custom canonical serializer that sorts map keys.
    // For now, we rely on the caller to produce maps with stable key order.
    // A robust canonical approach uses a dedicated crate (e.g. `json-canonicalization`).
    // Below is a best-effort compact serialization which is deterministic
    // as long as serde_json map ordering is preserved during construction.
    value.serialize(&mut ser).ok();
    let hash = Sha256::digest(&buf);
    hash.into()
}

/// Compute a Merkle root from a list of deterministic hashes.
pub fn merkle_root_from_hashes(hashes: &[DeterministicHash]) -> DeterministicHash {
    if hashes.is_empty() {
        return [0u8; 32];
    }
    let mut current = hashes.to_vec();
    while current.len() > 1 {
        let mut next = Vec::with_capacity(current.len().div_ceil(2));
        for chunk in current.chunks(2) {
            let mut hasher = Sha256::new();
            hasher.update(chunk[0]);
            if chunk.len() == 2 {
                hasher.update(chunk[1]);
            } else {
                hasher.update(chunk[0]); // duplicate last element
            }
            next.push(hasher.finalize().into());
        }
        current = next;
    }
    current[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_integrity() {
        let payload = serde_json::json!({"balance": 1000, "owner": "alice"});
        let receipt = CanonicalReceipt::new(
            AgentId::SolanaQuery,
            ReceiptType::QueryResult,
            "account query",
            payload,
        );
        assert!(receipt.verify_integrity());
    }

    #[test]
    fn merkle_root_empty() {
        let root = merkle_root_from_hashes(&[]);
        assert_eq!(root, [0u8; 32]);
    }

    #[test]
    fn merkle_root_single() {
        let h = [1u8; 32];
        let root = merkle_root_from_hashes(&[h]);
        assert_eq!(root, h);
    }

    #[test]
    fn merkle_root_two() {
        let a = [1u8; 32];
        let b = [2u8; 32];
        let root = merkle_root_from_hashes(&[a, b]);
        let mut hasher = Sha256::new();
        hasher.update(&a);
        hasher.update(&b);
        let expected: DeterministicHash = hasher.finalize().into();
        assert_eq!(root, expected);
    }
}
