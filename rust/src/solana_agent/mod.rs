//! Solana Agent — SQL-like queries and knowledge base for Solana blockchain

pub mod api;
pub mod auth;
pub mod cli;
pub mod decoder;
pub mod export;
pub mod knowledge_base;
pub mod query;
pub mod query_engine;
pub mod rpc_client;
pub mod storage;
pub mod stream;

#[cfg(test)]
mod tests;

pub use api::build_router;
pub use auth::{api_key_auth_middleware, rate_limit_middleware, AuthConfig, RateLimiter};
pub use cli::run_cli;
pub use decoder::{decode_account, decode_from_rpc_response, DecodedAccount};
pub use export::{export_query_result, ExportError, ExportFormat};
pub use knowledge_base::SolanaKnowledgeBase;
pub use query_engine::{QueryEngine, QueryResult};
pub use rpc_client::SolanaRpcClient;
pub use storage::{SolanaStorage, StorageError};
