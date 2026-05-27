//! Solana Agent — SQL-like queries and knowledge base for Solana blockchain

pub mod api;
pub mod cli;
pub mod knowledge_base;
pub mod query_engine;
pub mod rpc_client;

#[cfg(test)]
mod tests;

pub use api::build_router;
pub use cli::run_cli;
pub use knowledge_base::SolanaKnowledgeBase;
pub use query_engine::{QueryEngine, QueryResult};
pub use rpc_client::SolanaRpcClient;
