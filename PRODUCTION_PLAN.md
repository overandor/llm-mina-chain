# LLM-Mina-Chain Productionization Plan

## Current State

- `cargo build`: **0 errors, 0 warnings**
- `cargo test`: **70 tests pass, 0 failures**
- Deterministic block hashing and replay validation: **working**
- Real Ed25519 cryptography: **working**
- RocksDB storage: **working**
- Prometheus metrics: **working**
- Security layer (validation, rate limiting, audit logging): **working**
- Solana Agent (RPC client, query engine, Axum API): **working**
- Canonical Protocol Layer (receipts, config, types, logging): **established**

## Remaining Placeholder / Mocked Code

| Component | Status | Blocker |
|-----------|--------|---------|
| `zkproof.rs` | Placeholder proofs (zero-byte), placeholder VK hash | Needs real arkworks circuit + trusted setup |
| `consensus.rs` | Dummy signatures (all zeros), not wired to network | Needs real signing + network transport |
| `network.rs` | Gated behind `network` feature, libp2p API drift | Needs libp2p 0.53 API migration |
| `health.rs` | Hardcoded system metrics (512MB, 45% CPU) | Needs `sysinfo` or platform-specific calls |
| `llm_layer.rs` | Regex-only parsing, no LLM backend | Needs Ollama/local LLM HTTP client |
| `lib.rs generate_proof` | Deterministic string placeholder | Needs zkproof integration or removal |

---

## Agent Split & Roadmap

### Agent 1 — Core Runtime Stabilization
**Scope:** Rust compile correctness, networking, consensus, storage, replay, cryptography, deterministic execution, tests, benchmarks.
**Must NOT touch:** UI, product concepts, Solana queries, proof generation.

**Phase 1: Networking (P0)**
- [ ] Migrate `network.rs` to libp2p 0.53 stable API (SwarmBuilder, Behaviour, etc.)
- [ ] Remove `network` feature gate once it compiles standalone
- [ ] Add network integration tests (local multi-node bootstrap)
- [ ] Benchmark: 100 nodes, 1000 msgs, gossipsub latency

**Phase 2: Consensus (P0)**
- [ ] Replace dummy signatures in `consensus.rs` with real `DigitalSignature::sign()`
- [ ] Wire `HotStuffConsensus` into `Blockchain::create_block()`
- [ ] Add quorum certificate persistence to `BlockchainStorage`
- [ ] Test: 4 validators, byzantine fault tolerance (1 malicious node)

**Phase 3: Determinism & Replay (P1)**
- [ ] Add `Blockchain::replay()` method that rebuilds state from block history
- [ ] Add property test: `replay(chain) == current_state` for all valid chains
- [ ] Add benchmark: replay 10k blocks in < 1s

**Phase 4: Health & Observability (P1)**
- [ ] Replace placeholder `SystemMetrics` with `sysinfo` crate
- [ ] Expose health endpoint via `HealthChecker`
- [ ] Add disk-usage and memory-pressure alerts to metrics

---

### Agent 2 — Solana + Query + API Layer
**Scope:** Solana RPC, SQL-like query engine, analytics, REST APIs, Axum, RPC indexing, Dune-style query abstractions, API docs, endpoint stability, WebSocket streams.
**Must NOT touch:** core blockchain state, consensus messages, proof generation.

**Phase 1: RPC Robustness (P0)**
- [ ] Add connection pooling + retry logic to `SolanaRpcClient`
- [ ] Handle all Solana RPC error codes explicitly (rate limit, slot skipped, etc.)
- [ ] Add `getSignaturesForAddress` pagination support

**Phase 2: Query Engine (P0)**
- [ ] Add `JOIN` support across RPC endpoints (e.g., blocks + transactions)
- [ ] Add aggregation functions (`COUNT`, `SUM`, `AVG`) to `QueryEngine`
- [ ] Cache query results in `BlockchainStorage` (SQLite or RocksDB secondary index)
- [ ] Test: execute 50 different query patterns against mock RPC

**Phase 3: WebSocket Streams (P1)**
- [ ] Add `tokio-tungstenite` for Solana program account subscription streams
- [ ] Expose `/ws/transactions` endpoint for real-time transaction feed
- [ ] Add backpressure and client disconnection handling

**Phase 4: API Hardening (P1)**
- [ ] OpenAPI spec generation from `ApiRegistry`
- [ ] Request/response validation middleware
- [ ] API versioning strategy enforcement (`ApiVersion` headers)
- [ ] Load test: 1000 req/s on `/query` endpoint

---

### Agent 3 — Proof + Provenance + MemGas
**Scope:** Proof-of-Inference, Merkle chains, receipt generation, replay verification, IPFS, Solana anchoring, provenance, audit logs, memory collateral, deterministic proof state.
**Must NOT touch:** consensus logic, P2P networking, query engine SQL parsing.

**Phase 1: Real zk-SNARKs (P0)**
- [ ] Replace placeholder `ProofSystem::generate_proof()` with real `ark-groth16` circuit
- [ ] Implement `StateTransitionCircuit` as actual `ConstraintSynthesizer`
- [ ] Trusted setup: generate PK/VK in `ProofSystem::setup()`, store in `BlockchainStorage`
- [ ] Test: proof generation < 500ms, verification < 50ms for simple transfer circuit

**Phase 2: Receipt Pipeline (P0)**
- [ ] Wire `CanonicalReceipt` generation into every block commit
- [ ] Add `ReceiptType::Execution` receipts from Agent 1 block production
- [ ] Add `ReceiptType::QueryResult` receipts from Agent 2 query engine
- [ ] Add `ReceiptType::Proof` receipts from Agent 3 zkproof module
- [ ] Test: `hash_json_canonical(receipt1) == hash_json_canonical(receipt2)` for identical inputs

**Phase 3: Merkle + Anchoring (P1)**
- [ ] Build Merkle tree over block receipts after each block
- [ ] Persist Merkle root in block header
- [ ] Add IPFS upload for receipt bundles (optional feature `ipfs`)
- [ ] Add Solana anchoring: call `solana_agent::rpc_client` to write Merkle root as memo transaction

**Phase 4: Audit & Provenance (P1)**
- [ ] Add `AuditLogger::log_receipt()` to append-only audit log in RocksDB
- [ ] Implement cross-module receipt verification (signature + Merkle proof)
- [ ] Add `replay verify --receipt <hash>` CLI command

---

## Canonical Rules (enforced in code)

1. **Rust is authoritative runtime.** No other language may own block validation logic.
2. **All receipts use `CanonicalReceipt`** with `hash_json_canonical`.
3. **All hashes are SHA-256**, deterministic, and use canonical JSON with sorted keys.
4. **All APIs carry `ApiVersion`**; incompatible versions fail fast.
5. **No mock cryptography.** `DigitalSignature` must use `ed25519-dalek` everywhere.
6. **No placeholder responses.** Every method returns a real result or a typed error.
7. **No duplicate networking layers.** Only `network.rs` may use `libp2p`.
8. **No duplicate storage abstractions.** Only `BlockchainStorage` may use `rocksdb`.
9. **No silent fallback behavior.** Config uses explicit defaults; no hidden magic.
10. **No hidden autogenerated state.** All state changes are traceable to a transaction or block.

---

## Daily Integration Checkpoint

Every merge cycle must pass:

```bash
cd rust && cargo test && cargo clippy -- -D warnings && cargo build --release
```

If this fails, stop feature work until stability returns.

## Metrics for "Done"

| Criterion | Target |
|-----------|--------|
| `cargo test` | 100+ tests, 0 failures |
| `cargo clippy` | 0 warnings with `-D warnings` |
| Block replay | < 1ms per block |
| Proof generation | < 500ms |
| Proof verification | < 50ms |
| Query latency (p99) | < 100ms |
| Network gossip (p99) | < 50ms for 100 nodes |
| Consensus finality | < 2s for 4 validators |
