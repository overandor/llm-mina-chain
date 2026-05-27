use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Answer {
    pub answer: String,
    pub sources: Vec<String>,
    pub topic: String,
}

pub struct SolanaKnowledgeBase {
    topics: HashMap<String, Topic>,
}

pub struct Topic {
    keywords: Vec<String>,
    answer: String,
    sources: Vec<String>,
}

impl Default for SolanaKnowledgeBase {
    fn default() -> Self {
        Self::new()
    }
}

impl SolanaKnowledgeBase {
    pub fn new() -> Self {
        Self {
            topics: Self::build_topics(),
        }
    }

    pub fn ask(&self, question: &str) -> Option<Answer> {
        let q = question.to_lowercase();
        let mut best_topic: Option<String> = None;
        let mut best_score: usize = 0;

        for (name, topic) in &self.topics {
            let mut score = 0usize;
            for kw in &topic.keywords {
                if q.contains(kw) {
                    score += kw.len();
                }
            }
            if score > best_score {
                best_score = score;
                best_topic = Some(name.clone());
            }
        }

        if let Some(name) = best_topic {
            let topic = self.topics.get(&name).unwrap();
            Some(Answer {
                answer: topic.answer.clone(),
                sources: topic.sources.clone(),
                topic: name,
            })
        } else {
            None
        }
    }

    pub fn list_topics(&self) -> Vec<String> {
        self.topics.keys().cloned().collect()
    }

    pub fn get_topic(&self, name: &str) -> Option<&Topic> {
        self.topics.get(name)
    }

    fn build_topics() -> HashMap<String, Topic> {
        let mut m = HashMap::new();
        m.insert("architecture".into(), Topic {
            keywords: vec!["architecture", "design", "how does solana work", "proof of history", "poh", "tower bft", "gulf stream", "turbine", "sealevel", "pipeline", "cloudbreak", "archivers"].into_iter().map(|s| s.to_string()).collect(),
            answer: "Solana is a high-performance Layer 1 blockchain built around eight core innovations:\n1. Proof of History (PoH) — a cryptographic clock that orders transactions before consensus.\n2. Tower BFT — a PoH-optimized PBFT variant for faster finality.\n3. Gulf Stream — mempool-less transaction forwarding to upcoming leaders.\n4. Turbine — block propagation via erasure coding and random paths.\n5. Sealevel — parallel smart contract execution.\n6. Pipelining — optimized validation pipeline (fetch, verify, bank).\n7. Cloudbreak — horizontally-scalable accounts database.\n8. Archivers — distributed ledger storage with proofs.".into(),
            sources: vec!["https://solana.com/news/8-innovations-that-make-solana-the-first-web-scale-blockchain".into()],
        });
        m.insert("proof_of_history".into(), Topic {
            keywords: vec!["proof of history", "poh", "cryptographic clock", "verifiable delay function", "vdf", "time", "sequence"].into_iter().map(|s| s.to_string()).collect(),
            answer: "Proof of History (PoH) is Solana's core innovation — a sequence of computations that provides a way to cryptographically verify passage of time between two events. It uses a recursive, sequential SHA-256 hash where the output of one hash is the input for the next. Because SHA-256 is deterministic and sequential, validators can verify the entire sequence in parallel rather than re-computing it. This creates a 'cryptographic clock' that orders transactions before they enter consensus, dramatically reducing communication overhead in Tower BFT.".into(),
            sources: vec!["https://solana.com/news/proof-of-history-explained-by-anatoly-yakovenko".into()],
        });
        m.insert("accounts".into(), Topic {
            keywords: vec!["account", "accounts", "pubkey", "address", "owner", "data", "lamport", "rent"].into_iter().map(|s| s.to_string()).collect(),
            answer: "Solana uses an account-based model (not UTXO). Every account has a unique public key and stores: lamports (SOL balance, 1 SOL = 10^9 lamports), owner (the program that owns it), data (arbitrary state bytes), executable (whether it's a program), and rent_epoch. Accounts must pay rent or be rent-exempt. Native SOL accounts are owned by the System Program.".into(),
            sources: vec!["https://docs.solana.com/developing/programming-model/accounts".into()],
        });
        m.insert("transactions".into(), Topic {
            keywords: vec!["transaction", "tx", "instruction", "signature", "fee", "compute units", "cu", "priority fee", "jito", "mempool"].into_iter().map(|s| s.to_string()).collect(),
            answer: "Solana transactions contain one or more instructions, each specifying a program to invoke, accounts to read/write, and instruction data. Key concepts: base fee = 5000 lamports per signature; compute units (CUs) default to 1.4M per transaction; priority fees (micro-lamports per CU) help faster inclusion; no traditional mempool (Gulf Stream forwards to leaders); Jito provides MEV auction support; transactions are atomic.".into(),
            sources: vec!["https://docs.solana.com/developing/programming-model/transactions".into()],
        });
        m.insert("programs".into(), Topic {
            keywords: vec!["program", "smart contract", "contract", "deploy", "bpf", "ebpf", "rust", "anchor", "native program", "spl"].into_iter().map(|s| s.to_string()).collect(),
            answer: "Solana programs are compiled to eBPF bytecode and stored in executable accounts. Native Programs are built into the validator (System, Stake, Vote, etc.). SPL Programs include Token, ATA, Memo, etc. Custom Programs are written in Rust/C and compiled to BPF. Anchor is a popular Rust framework. Programs are stateless; all state lives in separate data accounts. Programs are upgraded by the upgrade authority.".into(),
            sources: vec!["https://docs.solana.com/developing/intro/program_overview".into()],
        });
        m.insert("tokens".into(), Topic {
            keywords: vec!["token", "spl token", "spl", "mint", "associated token account", "ata", "metadata", "metaplex", "nft", "fungible"].into_iter().map(|s| s.to_string()).collect(),
            answer: "Solana tokens use the SPL Token standard. Mint defines token properties (supply, decimals, freeze authority). Token Account holds balances for a specific mint and owner. Associated Token Account (ATA) is a deterministic address derived from owner + mint. Metaplex Token Metadata stores name/symbol/URI. Token-2022 adds extensions like confidential transfers and transfer fees.".into(),
            sources: vec!["https://spl.solana.com/token".into()],
        });
        m.insert("staking".into(), Topic {
            keywords: vec!["stake", "staking", "validator", "delegation", "epoch", "inflation", "reward", "vote account", "warmup", "cooldown"].into_iter().map(|s| s.to_string()).collect(),
            answer: "Solana uses Delegated Proof of Stake (dPoS). Stake Account holds delegated SOL with an authority controlling delegation and withdrawal. Vote Account is created by validators to receive delegations. Epoch is ~2 days; delegations take effect next epoch. Warmup/cooldown: stake activates/deactivates over an epoch. Inflation target ~8% initially, decreasing 15% per year until 1.5%. Rewards distributed automatically at epoch boundaries.".into(),
            sources: vec!["https://docs.solana.com/staking/stake-accounts".into()],
        });
        m.insert("consensus".into(), Topic {
            keywords: vec!["consensus", "tower bft", "bft", "finality", "slot", "epoch", "leader", "block producer", "vote", "optimistic confirmation", "rooted"].into_iter().map(|s| s.to_string()).collect(),
            answer: "Solana uses Tower BFT, a PBFT-like consensus optimized with PoH. Slot is ~400ms; one leader per slot produces up to 4 blocks (shreds). Epoch is ~432,000 slots (~2 days). Optimistic Confirmation: >66.66% stake votes. Rooted/Finalized: >33.33% stake votes on a descendant 32+ slots deeper. Leader Schedule computed per epoch from stake distribution. Slashing is not yet implemented.".into(),
            sources: vec!["https://docs.solana.com/cluster/commitments".into()],
        });
        m.insert("clusters".into(), Topic {
            keywords: vec!["mainnet", "devnet", "testnet", "cluster", "network", "rpc", "endpoint", "explorer"].into_iter().map(|s| s.to_string()).collect(),
            answer: "Solana clusters: Mainnet-Beta (production, real SOL), Devnet (test network with faucet), Testnet (validator/feature testing). Custom RPCs: QuickNode, Alchemy, Helius, Ankr. Explorers: Solscan, SolanaFM, Explorer.solana.com.".into(),
            sources: vec!["https://docs.solana.com/cluster/rpc-endpoints".into()],
        });
        m.insert("security".into(), Topic {
            keywords: vec!["security", "hack", "exploit", "reentrancy", "account validation", "ownership check", "signer check", "arbitrary cpi", "type confusion"].into_iter().map(|s| s.to_string()).collect(),
            answer: "Common Solana vulnerabilities: Missing Signer Check; Missing Ownership Check; Account Confusion/Validation; Reentrancy via CPI; Arithmetic Overflow; Arbitrary CPI; PDA closure issues. Best practice: Use Anchor constraints and practice with sealevel-attacker/CTF challenges.".into(),
            sources: vec!["https://docs.solana.com/developing/program-security".into()],
        });
        m.insert("program_derived_address".into(), Topic {
            keywords: vec!["pda", "program derived address", "program derived account", "bump", "seed", "find_program_address", "create_program_address"].into_iter().map(|s| s.to_string()).collect(),
            answer: "Program Derived Addresses (PDAs) are deterministically derived from a program ID and seeds. They have no private key; only the owning program can sign via CPI. find_program_address iterates a bump seed (0-255) until a valid off-curve address is found. create_program_address computes directly if bump is known. Use cases: escrow, vaults, user data, authorities.".into(),
            sources: vec!["https://docs.solana.com/developing/programming-model/calling-between-programs#program-derived-addresses".into()],
        });
        m.insert("compression".into(), Topic {
            keywords: vec!["compression", "state compression", "compressed nft", "cnft", "merkle tree", "bubblegum", "light"].into_iter().map(|s| s.to_string()).collect(),
            answer: "State Compression (compressed NFTs / cNFTs) uses concurrent Merkle trees to store data off-chain while anchoring a small Merkle root on-chain. Bubblegum (Metaplex) is the primary program. Cost: ~0.0001 SOL per cNFT vs ~0.012 SOL uncompressed. RPC providers must index the tree to serve proofs.".into(),
            sources: vec!["https://docs.solana.com/developing/plugins/state-compression".into()],
        });
        m.insert("fees".into(), Topic {
            keywords: vec!["fee", "gas", "priority fee", "compute budget", "cu", "cost", "cheap", "expensive", "rent"].into_iter().map(|s| s.to_string()).collect(),
            answer: "Solana fee structure: base transaction fee = 5000 lamports per signature (0.000005 SOL). Priority Fee = optional micro-lamports per CU. Compute Budget = default 1.4M CUs per transaction. Rent = minimum balance to persist accounts. Simple transfer: ~0.000005 SOL. Complex DeFi swap: ~0.00002-0.001 SOL depending on priority fees.".into(),
            sources: vec!["https://docs.solana.com/transaction_fees".into()],
        });
        m
    }
}
