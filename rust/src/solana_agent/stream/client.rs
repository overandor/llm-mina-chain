use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{broadcast, mpsc};
use tokio_tungstenite::{connect_async, tungstenite::Message};

use super::types::{StreamEvent, SubscriptionParams, SubscriptionType};

/// WebSocket client for Solana streaming RPC subscriptions.
pub struct SolanaStreamClient {
    endpoint: String,
    tx: Option<mpsc::Sender<Value>>,
    broadcast: Option<broadcast::Sender<StreamEvent>>,
    id_counter: AtomicU64,
}

impl SolanaStreamClient {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            tx: None,
            broadcast: None,
            id_counter: AtomicU64::new(1),
        }
    }

    /// Convert HTTP endpoint to WSS endpoint.
    fn ws_endpoint(&self) -> String {
        self.endpoint
            .replace("https://", "wss://")
            .replace("http://", "ws://")
    }

    /// Connect and spawn a background read loop.
    pub async fn connect(&mut self) -> Result<(), StreamError> {
        let url = self.ws_endpoint();
        let (ws_stream, _) = connect_async(&url).await.map_err(StreamError::from)?;

        let (mut write, mut read) = ws_stream.split();
        let (tx_to_ws, mut rx_to_ws) = mpsc::channel::<Value>(100);
        let (broadcast_tx, _) = broadcast::channel(1000);

        // Spawn writer task
        tokio::spawn(async move {
            while let Some(msg) = rx_to_ws.recv().await {
                let text = serde_json::to_string(&msg).unwrap_or_default();
                if write.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
        });

        let bcast = broadcast_tx.clone();
        // Spawn reader task
        tokio::spawn(async move {
            while let Some(Ok(msg)) = read.next().await {
                if let Message::Text(text) = msg {
                    if let Ok(json) = serde_json::from_str::<Value>(&text) {
                        // Skip subscription acknowledgement messages
                        if json.get("result").is_some() {
                            continue;
                        }
                        if let Some(params) = json.get("params") {
                            let slot = params.get("result")
                                .and_then(|r| r.get("slot"))
                                .and_then(|s| s.as_u64());
                            // Try to infer subscription type from method if present
                            let sub_type = SubscriptionType::Slot; // default; refine if needed
                            let event = StreamEvent {
                                subscription: sub_type,
                                payload: params.clone(),
                                slot,
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64,
                            };
                            let _ = bcast.send(event);
                        }
                    }
                }
            }
        });

        self.tx = Some(tx_to_ws);
        self.broadcast = Some(broadcast_tx);
        Ok(())
    }

    fn next_id(&self) -> u64 {
        self.id_counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Send a subscription request and return a broadcast receiver for events.
    pub async fn subscribe(
        &self,
        sub_type: SubscriptionType,
        params: Option<SubscriptionParams>,
    ) -> Result<broadcast::Receiver<StreamEvent>, StreamError> {
        let tx = self.tx.as_ref().ok_or(StreamError::NotConnected)?;
        let bcast = self.broadcast.as_ref().ok_or(StreamError::NotConnected)?;

        let id = self.next_id();
        let mut req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": sub_type.method(),
        });

        if let Some(p) = params {
            match p {
                SubscriptionParams::Account { pubkey, commitment } => {
                    req["params"] = json!([pubkey, {"commitment": commitment, "encoding": "base64"}]);
                }
                SubscriptionParams::Program { program_id, commitment } => {
                    req["params"] = json!([program_id, {"commitment": commitment, "encoding": "base64"}]);
                }
                SubscriptionParams::Signature { signature, commitment } => {
                    req["params"] = json!([signature, {"commitment": commitment}]);
                }
                SubscriptionParams::Logs { filter } => {
                    req["params"] = json!([{"mentions": filter.mentions}]);
                }
                SubscriptionParams::Slot {} | SubscriptionParams::Root {} => {
                    req["params"] = json!([]);
                }
            }
        } else {
            req["params"] = json!([]);
        }

        tx.send(req).await.map_err(|_| StreamError::SendError)?;
        Ok(bcast.subscribe())
    }

    /// Subscribe to slot notifications.
    pub async fn subscribe_slots(&self) -> Result<broadcast::Receiver<StreamEvent>, StreamError> {
        self.subscribe(SubscriptionType::Slot, Some(SubscriptionParams::Slot {})).await
    }

    /// Subscribe to account changes.
    pub async fn subscribe_account(
        &self,
        pubkey: &str,
        commitment: &str,
    ) -> Result<broadcast::Receiver<StreamEvent>, StreamError> {
        self.subscribe(
            SubscriptionType::Account,
            Some(SubscriptionParams::Account {
                pubkey: pubkey.to_string(),
                commitment: commitment.to_string(),
            }),
        )
        .await
    }

    /// Subscribe to program account changes.
    pub async fn subscribe_program(
        &self,
        program_id: &str,
        commitment: &str,
    ) -> Result<broadcast::Receiver<StreamEvent>, StreamError> {
        self.subscribe(
            SubscriptionType::Program,
            Some(SubscriptionParams::Program {
                program_id: program_id.to_string(),
                commitment: commitment.to_string(),
            }),
        )
        .await
    }

    /// Subscribe to logs mentioning specific accounts/programs.
    pub async fn subscribe_logs(
        &self,
        mentions: Vec<String>,
    ) -> Result<broadcast::Receiver<StreamEvent>, StreamError> {
        self.subscribe(
            SubscriptionType::Logs,
            Some(SubscriptionParams::Logs {
                filter: super::types::LogsFilter { mentions },
            }),
        )
        .await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("WebSocket not connected")]
    NotConnected,
    #[error("Send error")]
    SendError,
    #[error("WebSocket error: {0}")]
    WsError(#[from] tokio_tungstenite::tungstenite::Error),
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}
