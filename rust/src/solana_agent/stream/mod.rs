//! Solana WebSocket Streaming Subscriptions
//!
//! Provides real-time streaming of blockchain events:
//! - slotSubscribe — new slot notifications
//! - accountSubscribe — account state changes
//! - programSubscribe — program account changes
//! - logsSubscribe — transaction log filtering
//! - signatureSubscribe — signature confirmation
//!
//! Example:
//! ```rust,ignore
//! let mut client = SolanaStreamClient::new("wss://api.mainnet-beta.solana.com".into());
//! client.connect().await.unwrap();
//! let mut rx = client.subscribe_slots().await.unwrap();
//! while let Some(event) = rx.recv().await {
//!     println!("New slot: {:?}", event.payload);
//! }
//! ```

pub mod client;
pub mod types;

pub use client::{SolanaStreamClient, StreamError};
pub use types::{LogsFilter, StreamEvent, SubscriptionParams, SubscriptionType};
