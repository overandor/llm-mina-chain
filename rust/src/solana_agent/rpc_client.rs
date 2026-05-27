use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::sync::atomic::{AtomicU64, Ordering};

static PUBLIC_RPCS: &[&str] = &[
    "https://api.mainnet-beta.solana.com",
    "https://solana-api.projectserum.com",
    "https://rpc.ankr.com/solana",
];

pub struct SolanaRpcClient {
    endpoint: String,
    client: Client,
    id_counter: AtomicU64,
}

impl Clone for SolanaRpcClient {
    fn clone(&self) -> Self {
        Self {
            endpoint: self.endpoint.clone(),
            client: self.client.clone(),
            id_counter: AtomicU64::new(0),
        }
    }
}

impl SolanaRpcClient {
    pub fn new(endpoint: Option<String>) -> Self {
        let endpoint = endpoint
            .or_else(|| env::var("SOLANA_RPC_ENDPOINT").ok())
            .unwrap_or_else(|| PUBLIC_RPCS[0].to_string());
        Self {
            endpoint,
            client: Client::new(),
            id_counter: AtomicU64::new(0),
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value, RpcError> {
        let id = self.id_counter.fetch_add(1, Ordering::SeqCst);
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let resp = self
            .client
            .post(&self.endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|e| RpcError::Transport(e.to_string()))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .map_err(|e| RpcError::Transport(e.to_string()))?;

        if !status.is_success() {
            return Err(RpcError::Http(status.as_u16(), text));
        }

        let json_resp: RpcResponse = serde_json::from_str(&text)
            .map_err(|e| RpcError::Parse(e.to_string()))?;

        if let Some(err) = json_resp.error {
            return Err(RpcError::Rpc(err.code, err.message));
        }

        Ok(json_resp.result.unwrap_or(Value::Null))
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
