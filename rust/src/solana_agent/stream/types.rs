use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A Solana WebSocket subscription event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    /// Subscription type
    pub subscription: SubscriptionType,
    /// Raw JSON payload from Solana
    pub payload: Value,
    /// Slot at which the event was observed
    pub slot: Option<u64>,
    /// Timestamp (Unix millis)
    pub timestamp: u64,
}

/// Discriminant for subscription types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriptionType {
    Slot,
    Account,
    Program,
    Logs,
    Signature,
    Root,
}

impl SubscriptionType {
    pub fn method(&self) -> &'static str {
        match self {
            SubscriptionType::Slot => "slotSubscribe",
            SubscriptionType::Account => "accountSubscribe",
            SubscriptionType::Program => "programSubscribe",
            SubscriptionType::Logs => "logsSubscribe",
            SubscriptionType::Signature => "signatureSubscribe",
            SubscriptionType::Root => "rootSubscribe",
        }
    }

    pub fn unsubscribe_method(&self) -> &'static str {
        match self {
            SubscriptionType::Slot => "slotUnsubscribe",
            SubscriptionType::Account => "accountUnsubscribe",
            SubscriptionType::Program => "programUnsubscribe",
            SubscriptionType::Logs => "logsUnsubscribe",
            SubscriptionType::Signature => "signatureUnsubscribe",
            SubscriptionType::Root => "rootUnsubscribe",
        }
    }
}

/// Filter for logs subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsFilter {
    pub mentions: Vec<String>,
}

/// Subscription request parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SubscriptionParams {
    Account { pubkey: String, commitment: String },
    Program { program_id: String, commitment: String },
    Signature { signature: String, commitment: String },
    Logs { filter: LogsFilter },
    Slot {},
    Root {},
}
