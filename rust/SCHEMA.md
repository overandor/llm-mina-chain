# Canonical Schema & Receipt Format

This document is the single source of truth for all serialized data structures.
**No agent may change this without three-agent review.**

## 1. Transaction Receipt

Every applied transaction produces exactly one receipt.

```json
{
  "receipt_version": "1.0.0",
  "receipt_id": "sha256-hex-64-chars",
  "tx": {
    "tx_id": "string",
    "sender": "string",
    "receiver": "string",
    "amount": "u64",
    "nonce": "u64",
    "gas_limit": "Option<u64>",
    "gas_price": "Option<u64>",
    "tx_type": "string",
    "data": "Option<serde_json::Value>",
    "signature": "Option<hex-128-chars>",
    "timestamp": "i64"
  },
  "pre_state_root": "sha256-hex-64-chars",
  "post_state_root": "sha256-hex-64-chars",
  "gas_used": "u64",
  "success": "bool",
  "block_height": "u64",
  "block_hash": "sha256-hex-64-chars",
  "agent_signature": "hex-128-chars",
  "agent_public_key": "hex-64-chars"
}
```

## 2. State Commitment (Merkle Root)

The canonical state hash is a Merkle root over:
1. Sorted balances map
2. Sorted nonces map
3. Chain metadata

Serialization order (deterministic):
- Keys sorted lexicographically
- Values as big-endian bytes
- Concatenated, then SHA-256

## 3. Block Header

```json
{
  "version": 1,
  "height": "u64",
  "timestamp": "i64",
  "prev_hash": "sha256-hex-64",
  "state_root": "sha256-hex-64",
  "tx_root": "sha256-hex-64",
  "extra_data": "hex"
}
```

`tx_root` is the Merkle root of all transaction hashes in the block.

## 4. Proof-of-Inference Receipt

```json
{
  "proof_version": "1.0.0",
  "inference_hash": "sha256-hex-64",
  "input_hash": "sha256-hex-64",
  "output_hash": "sha256-hex-64",
  "model_id": "string",
  "timestamp": "i64",
  "merkle_path": ["sha256-hex-64"],
  "anchoring_tx": "Option<string>"
}
```

## 5. Hash Algorithm

- **SHA-256** everywhere.
- **Hex encoding**: lowercase, no `0x` prefix.
- **Deterministic**: same inputs always produce same output.

## 6. Serialization Rules

- `serde_json` with `SortMaps` (canonical key order).
- No pretty-printing (compact JSON).
- No extra fields.
- Timestamps are Unix seconds (i64).

## 7. Versioning

- All schemas are versioned with SemVer.
- Backward-incompatible changes require a new major version.
- Old versions must be readable for at least 30 days.
