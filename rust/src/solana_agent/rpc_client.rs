use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, instrument, warn};

static PUBLIC_RPCS: &[&str] = &[
    "https://api.mainnet-beta.solana.com",
    "https://solana-api.projectserum.com",
    "https://rpc.ankr.com/solana",
];

/// Health score for an RPC endpoint
#[derive(Debug, Clone)]
struct EndpointHealth {
    endpoint: String,
    success_count: u64,
    failure_count: u64,
    last_latency_ms: Option<u64>,
    last_check: Option<Instant>,
    is_healthy: bool,
}

impl EndpointHealth {
    fn new(endpoint: String) -> Self {
        Self {
            endpoint,
            success_count: 0,
            failure_count: 0,
            last_latency_ms: None,
            last_check: None,
            is_healthy: true,
        }
    }

    fn score(&self) -> f64 {
        if !self.is_healthy {
            return 0.0;
        }

        let total_requests = self.success_count + self.failure_count;
        if total_requests == 0 {
            return 100.0; // New endpoint gets max score
        }

        let success_rate = self.success_count as f64 / total_requests as f64;
        let latency_score = match self.last_latency_ms {
            Some(lat) => (1000.0 / (lat as f64 + 100.0)).min(1.0),
            None => 0.5,
        };

        (success_rate * 0.7 + latency_score * 0.3) * 100.0
    }

    fn record_success(&mut self, latency_ms: u64) {
        self.success_count += 1;
        self.last_latency_ms = Some(latency_ms);
        self.last_check = Some(Instant::now());
        self.is_healthy = true;
    }

    fn record_failure(&mut self) {
        self.failure_count += 1;
        self.last_check = Some(Instant::now());

        // Mark unhealthy if failure rate > 50% and > 5 failures
        let total = self.success_count + self.failure_count;
        if total > 5 && (self.failure_count as f64 / total as f64) > 0.5 {
            self.is_healthy = false;
        }
    }

    fn should_retry(&self) -> bool {
        if self.is_healthy {
            return true;
        }

        // Retry unhealthy endpoints after 5 minutes
        if let Some(last) = self.last_check {
            last.elapsed() > Duration::from_secs(300)
        } else {
            true
        }
    }
}

pub struct SolanaRpcClient {
    endpoints: Arc<RwLock<Vec<EndpointHealth>>>,
    client: Client,
    id_counter: AtomicU64,
}

impl Clone for SolanaRpcClient {
    fn clone(&self) -> Self {
        Self {
            endpoints: Arc::clone(&self.endpoints),
            client: self.client.clone(),
            id_counter: AtomicU64::new(0),
        }
    }
}

impl SolanaRpcClient {
    pub fn new(endpoint: Option<String>) -> Self {
        let endpoints = if let Some(custom) = endpoint.or_else(|| env::var("SOLANA_RPC_ENDPOINT").ok()) {
            vec![EndpointHealth::new(custom)]
        } else {
            PUBLIC_RPCS.iter().map(|s| EndpointHealth::new(s.to_string())).collect()
        };

        Self {
            endpoints: Arc::new(RwLock::new(endpoints)),
            client: Client::new(),
            id_counter: AtomicU64::new(0),
        }
    }

    pub fn endpoint(&self) -> String {
        let endpoints = self.endpoints.blocking_read();
        // Return the highest-scoring endpoint
        endpoints
            .iter()
            .max_by(|a, b| a.score().partial_cmp(&b.score()).unwrap())
            .map(|h| h.endpoint.clone())
            .unwrap_or_else(|| PUBLIC_RPCS[0].to_string())
    }

    async fn record_success(&self, endpoint: &str, latency_ms: u64) {
        let mut endpoints = self.endpoints.write().await;
        if let Some(health) = endpoints.iter_mut().find(|h| h.endpoint == endpoint) {
            health.record_success(latency_ms);
        }
    }

    async fn record_failure(&self, endpoint: &str) {
        let mut endpoints = self.endpoints.write().await;
        if let Some(health) = endpoints.iter_mut().find(|h| h.endpoint == endpoint) {
            health.record_failure();
        }
    }

    #[instrument(skip(self, params), fields(method = %method))]
    pub async fn call(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        let id = self.id_counter.fetch_add(1, Ordering::SeqCst);
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        debug!("RPC request: {} with params: {}", method, params);

        // Try each endpoint with failover
        let mut last_error = None;
        let endpoints = self.endpoints.read().await;

        for health in endpoints.iter() {
            if !health.should_retry() {
                warn!("Skipping unhealthy endpoint: {} (score: {:.1})", health.endpoint, health.score());
                continue;
            }

            let endpoint = health.endpoint.clone();
            let score = health.score();
            debug!("Trying endpoint: {} (score: {:.1})", endpoint, score);

            let start = std::time::Instant::now();
            let result = self
                .client
                .post(&endpoint)
                .json(&body)
                .send()
                .await;

            let elapsed = start.elapsed();

            match result {
                Ok(resp) => {
                    let status = resp.status();
                    let text = match resp.text().await {
                        Ok(t) => t,
                        Err(e) => {
                            error!("RPC response read error for {}: {}", method, e);
                            self.record_failure(&endpoint).await;
                            last_error = Some(RpcError::Transport(e.to_string()));
                            continue;
                        }
                    };

                    debug!("RPC response for {} from {} in {:?}", method, endpoint, elapsed);

                    if !status.is_success() {
                        error!("RPC HTTP error for {}: {} - {}", method, status, text);
                        self.record_failure(&endpoint).await;
                        last_error = Some(RpcError::Http(status.as_u16(), text));
                        continue;
                    }

                    let json_resp: RpcResponse = match serde_json::from_str(&text) {
                        Ok(r) => r,
                        Err(e) => {
                            error!("RPC parse error for {}: {}", method, e);
                            self.record_failure(&endpoint).await;
                            last_error = Some(RpcError::Parse(e.to_string()));
                            continue;
                        }
                    };

                    if let Some(err) = json_resp.error {
                        error!("RPC error for {}: code={}, message={}", method, err.code, err.message);
                        self.record_failure(&endpoint).await;
                        last_error = Some(RpcError::Rpc(err.code, err.message));
                        continue;
                    }

                    // Success - record health and return
                    self.record_success(&endpoint, elapsed.as_millis() as u64).await;
                    info!("RPC success: {} from {} in {:?}", method, endpoint, elapsed);
                    return Ok(json_resp.result.unwrap_or(Value::Null));
                }
                Err(e) => {
                    error!("RPC transport error for {} from {}: {}", method, endpoint, e);
                    self.record_failure(&endpoint).await;
                    last_error = Some(RpcError::Transport(e.to_string()));
                    continue;
                }
            }
        }

        // All endpoints failed
        error!("All RPC endpoints failed for method: {}", method);
        Err(last_error.unwrap_or_else(|| RpcError::Transport("No available endpoints".to_string())))
    }

    // ---- Account methods ----

    pub async fn get_account_info(
        &self,
        pubkey: &str,
        commitment: &str,
    ) -> Result<Value, RpcError> {
        self.call(
            "getAccountInfo",
            json!([pubkey, {"commitment": commitment, "encoding": "jsonParsed"}]),
        )
        .await
    }

    pub async fn get_balance(&self, pubkey: &str, commitment: &str) -> Result<u64, RpcError> {
        let r = self
            .call("getBalance", json!([pubkey, {"commitment": commitment}]))
            .await?;
        Ok(r["value"].as_u64().unwrap_or(0))
    }

    pub async fn get_multiple_accounts(
        &self,
        pubkeys: &[String],
        commitment: &str,
    ) -> Result<Value, RpcError> {
        self.call(
            "getMultipleAccounts",
            json!([pubkeys, {"commitment": commitment, "encoding": "jsonParsed"}]),
        )
        .await
    }

    // ---- Transaction methods ----

    pub async fn get_transaction(
        &self,
        signature: &str,
        commitment: &str,
    ) -> Result<Value, RpcError> {
        self.call(
            "getTransaction",
            json!([
                signature,
                {"commitment": commitment, "maxSupportedTransactionVersion": 0}
            ]),
        )
        .await
    }

    pub async fn get_signature_statuses(
        &self,
        signatures: &[String],
    ) -> Result<Value, RpcError> {
        self.call("getSignatureStatuses", json!([signatures])).await
    }

    // ---- Block methods ----

    pub async fn get_block(&self, slot: u64, commitment: &str) -> Result<Value, RpcError> {
        self.call(
            "getBlock",
            json!([
                slot,
                {
                    "commitment": commitment,
                    "maxSupportedTransactionVersion": 0,
                    "transactionDetails": "full",
                    "rewards": true
                }
            ]),
        )
        .await
    }

    pub async fn get_block_height(&self, commitment: &str) -> Result<u64, RpcError> {
        let r = self.call("getBlockHeight", json!([{"commitment": commitment}])).await?;
        Ok(r.as_u64().unwrap_or(0))
    }

    pub async fn get_block_time(&self, slot: u64) -> Result<Option<i64>, RpcError> {
        let r = self.call("getBlockTime", json!([slot])).await?;
        Ok(r.as_i64())
    }

    pub async fn get_blocks(
        &self,
        start_slot: u64,
        end_slot: Option<u64>,
    ) -> Result<Vec<u64>, RpcError> {
        let params = if let Some(end) = end_slot {
            json!([start_slot, end])
        } else {
            json!([start_slot])
        };
        let r = self.call("getBlocks", params).await?;
        Ok(serde_json::from_value(r).unwrap_or_default())
    }

    // ---- Slot / Epoch ----

    pub async fn get_slot(&self, commitment: &str) -> Result<u64, RpcError> {
        let r = self.call("getSlot", json!([{"commitment": commitment}])).await?;
        Ok(r.as_u64().unwrap_or(0))
    }

    pub async fn get_epoch_info(&self) -> Result<Value, RpcError> {
        self.call("getEpochInfo", json!([])).await
    }

    pub async fn get_epoch_schedule(&self) -> Result<Value, RpcError> {
        self.call("getEpochSchedule", json!([])).await
    }

    // ---- Token methods ----

    pub async fn get_token_accounts_by_owner(
        &self,
        owner: &str,
        mint: Option<&str>,
        commitment: &str,
    ) -> Result<Value, RpcError> {
        let filter = if let Some(m) = mint {
            json!({"mint": m})
        } else {
            json!({"programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"})
        };
        self.call(
            "getTokenAccountsByOwner",
            json!([owner, filter, {"commitment": commitment, "encoding": "jsonParsed"}]),
        )
        .await
    }

    pub async fn get_token_supply(
        &self,
        mint: &str,
        commitment: &str,
    ) -> Result<Value, RpcError> {
        let r = self
            .call(
                "getTokenSupply",
                json!([mint, {"commitment": commitment, "encoding": "jsonParsed"}]),
            )
            .await?;
        Ok(r["value"].clone())
    }

    pub async fn get_token_largest_accounts(
        &self,
        mint: &str,
        commitment: &str,
    ) -> Result<Value, RpcError> {
        let r = self
            .call("getTokenLargestAccounts", json!([mint, {"commitment": commitment}]))
            .await?;
        Ok(r["value"].clone())
    }

    // ---- Program methods ----

    pub async fn get_program_accounts(
        &self,
        program_id: &str,
        filters: Option<Value>,
        commitment: &str,
    ) -> Result<Value, RpcError> {
        let mut config = json!({"commitment": commitment, "encoding": "base64"});
        if let Some(f) = filters {
            config["filters"] = f;
        }
        self.call("getProgramAccounts", json!([program_id, config])).await
    }

    // ---- Supply / Inflation ----

    pub async fn get_supply(
        &self,
        commitment: &str,
        exclude_non_circulating: bool,
    ) -> Result<Value, RpcError> {
        let r = self
            .call(
                "getSupply",
                json!([{
                    "commitment": commitment,
                    "excludeNonCirculatingAccountsList": exclude_non_circulating
                }]),
            )
            .await?;
        Ok(r["value"].clone())
    }

    pub async fn get_inflation_reward(
        &self,
        pubkeys: &[String],
        epoch: Option<u64>,
    ) -> Result<Value, RpcError> {
        let mut params = json!([pubkeys]);
        if let Some(e) = epoch {
            params.as_array_mut().unwrap().push(json!({"epoch": e}));
        }
        self.call("getInflationReward", params).await
    }

    // ---- Performance ----

    pub async fn get_recent_performance_samples(&self, limit: u64) -> Result<Value, RpcError> {
        self.call("getRecentPerformanceSamples", json!([limit])).await
    }

    pub async fn get_cluster_nodes(&self) -> Result<Value, RpcError> {
        self.call("getClusterNodes", json!([])).await
    }

    pub async fn get_vote_accounts(&self) -> Result<Value, RpcError> {
        self.call("getVoteAccounts", json!([])).await
    }

    pub async fn get_leader_schedule(&self, slot: Option<u64>) -> Result<Value, RpcError> {
        let params = if let Some(s) = slot { json!([s]) } else { json!([]) };
        self.call("getLeaderSchedule", params).await
    }

    // ---- Fees ----

    pub async fn get_fees(&self, commitment: &str) -> Result<Value, RpcError> {
        let r = self
            .call("getFees", json!([{"commitment": commitment}]))
            .await?;
        Ok(r["value"].clone())
    }

    pub async fn get_fee_calculator_for_blockhash(
        &self,
        blockhash: &str,
    ) -> Result<Value, RpcError> {
        let r = self
            .call("getFeeCalculatorForBlockhash", json!([blockhash]))
            .await?;
        Ok(r["value"].clone())
    }

    pub async fn get_recent_blockhash(&self, commitment: &str) -> Result<Value, RpcError> {
        let r = self
            .call("getRecentBlockhash", json!([{"commitment": commitment}]))
            .await?;
        Ok(r["value"].clone())
    }

    // ---- Health / Meta ----

    pub async fn get_health(&self) -> Result<String, RpcError> {
        let r = self.call("getHealth", json!([])).await?;
        Ok(r.as_str().unwrap_or("unknown").to_string())
    }

    pub async fn get_version(&self) -> Result<Value, RpcError> {
        self.call("getVersion", json!([])).await
    }

    pub async fn get_identity(&self) -> Result<String, RpcError> {
        let r = self.call("getIdentity", json!([])).await?;
        Ok(r["identity"].as_str().unwrap_or("").to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RpcErrorObject {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RpcResponse {
    jsonrpc: String,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<RpcErrorObject>,
    id: Value,
}

#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    #[error("HTTP {0}: {1}")]
    Http(u16, String),
    #[error("RPC error {0}: {1}")]
    Rpc(i64, String),
    #[error("Transport: {0}")]
    Transport(String),
    #[error("Parse: {0}")]
    Parse(String),
}
