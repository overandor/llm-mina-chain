use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Instant;

use super::rpc_client::{RpcError, SolanaRpcClient};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub row_count: usize,
    pub execution_time_ms: f64,
    pub query_type: String,
}

pub struct QueryEngine {
    client: SolanaRpcClient,
}

#[derive(Debug, Default)]
struct ParsedQuery {
    select: Vec<String>,
    table: String,
    where_clauses: HashMap<String, String>,
    limit: Option<usize>,
}

impl QueryEngine {
    pub fn new(client: SolanaRpcClient) -> Self {
        Self { client }
    }

    pub async fn execute(
        &self,
        query: &str,
        params: Option<HashMap<String, String>>,
    ) -> Result<QueryResult, RpcError> {
        let start = Instant::now();
        let parsed = Self::parse(query);

        // substitute params
        let mut where_clauses = parsed.where_clauses.clone();
        if let Some(p) = params {
            for (_k, v) in &mut where_clauses {
                if v.starts_with(':') {
                    let key = v.trim_start_matches(':');
                    if let Some(replacement) = p.get(key) {
                        *v = replacement.clone();
                    }
                }
            }
        }

        let (columns, rows) = match parsed.table.as_str() {
            "accounts" => self.query_accounts(&where_clauses, &parsed.select).await?,
            "transactions" => self.query_transactions(&where_clauses, &parsed.select).await?,
            "blocks" => self.query_blocks(&where_clauses, &parsed.select).await?,
            "token_accounts" => {
                self.query_token_accounts(&where_clauses, &parsed.select).await?
            }
            "program_accounts" => {
                self.query_program_accounts(&where_clauses, &parsed.select).await?
            }
            "status" => self.query_status(&parsed.select).await?,
            "epoch_info" => self.query_epoch_info(&parsed.select).await?,
            "supply" => self.query_supply(&parsed.select).await?,
            "vote_accounts" => self.query_vote_accounts(&parsed.select).await?,
            "cluster_nodes" => self.query_cluster_nodes(&parsed.select).await?,
            "performance_samples" => {
                self.query_performance_samples(parsed.limit.unwrap_or(10), &parsed.select)
                    .await?
            }
            "token_supply" => self.query_token_supply(&where_clauses, &parsed.select).await?,
            "inflation_reward" => {
                self.query_inflation_reward(&where_clauses, &parsed.select).await?
            }
            "health" => self.query_health(&parsed.select).await?,
            "version" => self.query_version(&parsed.select).await?,
            _ => return Err(RpcError::Transport(format!("Unknown table: {}", parsed.table))),
        };

        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        let row_count = rows.len();
        Ok(QueryResult {
            columns,
            rows,
            row_count,
            execution_time_ms: elapsed,
            query_type: parsed.table,
        })
    }

    fn parse(query: &str) -> ParsedQuery {
        let q = query.trim().to_lowercase();
        let q = Regex::new(r"\s+").unwrap().replace_all(&q, " ");
        let mut parsed = ParsedQuery::default();

        // SELECT
        if let Some(caps) = Regex::new(r"select\s+(.+?)\s+from\s").unwrap().captures(&q) {
            let raw = caps.get(1).unwrap().as_str().trim();
            parsed.select = if raw == "*" {
                vec!["*".to_string()]
            } else {
                raw.split(',').map(|s| s.trim().to_string()).collect()
            };
        }

        // FROM
        if let Some(caps) = Regex::new(r"from\s+(\w+)").unwrap().captures(&q) {
            parsed.table = caps.get(1).unwrap().as_str().to_string();
        }

        // WHERE
        if let Some(caps) = Regex::new(r"where\s+(.+?)(?:\s+limit\s+|$)").unwrap().captures(&q) {
            let raw = caps.get(1).unwrap().as_str().trim();
            for cond in raw.split(" and ") {
                if let Some(caps) = Regex::new(r"(\w+)\s*=\s*(.+)").unwrap().captures(cond.trim()) {
                    let key = caps.get(1).unwrap().as_str().to_string();
                    let val = caps
                        .get(2)
                        .unwrap()
                        .as_str()
                        .trim()
                        .trim_matches('\'')
                        .trim_matches('"')
                        .to_string();
                    parsed.where_clauses.insert(key, val);
                }
            }
        }

        // LIMIT
        if let Some(caps) = Regex::new(r"limit\s+(\d+)").unwrap().captures(&q) {
            parsed.limit = caps.get(1).unwrap().as_str().parse().ok();
        }

        parsed
    }

    // ---- Query implementations ----

    async fn query_accounts(
        &self,
        where_clauses: &HashMap<String, String>,
        select: &[String],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), RpcError> {
        let pubkey = where_clauses
            .get("pubkey")
            .ok_or_else(|| RpcError::Transport("accounts requires pubkey".into()))?;
        let info = self.client.get_account_info(pubkey, "confirmed").await?;
        let value = info["value"].clone();
        if value.is_null() {
            return Ok((vec!["pubkey".into()], vec![]));
        }
        let row = flatten_account(pubkey, &value);
        let columns = pick_columns(select, &row);
        let values = columns.iter().map(|c| row.get(c).cloned().unwrap_or(Value::Null)).collect();
        Ok((columns, vec![values]))
    }

    async fn query_transactions(
        &self,
        where_clauses: &HashMap<String, String>,
        select: &[String],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), RpcError> {
        let sig = where_clauses
            .get("signature")
            .ok_or_else(|| RpcError::Transport("transactions requires signature".into()))?;
        let tx = self.client.get_transaction(sig, "confirmed").await?;
        if tx.is_null() {
            return Ok((vec!["signature".into()], vec![]));
        }
        let row = flatten_transaction(sig, &tx);
        let columns = pick_columns(select, &row);
        let values = columns.iter().map(|c| row.get(c).cloned().unwrap_or(Value::Null)).collect();
        Ok((columns, vec![values]))
    }

    async fn query_blocks(
        &self,
        where_clauses: &HashMap<String, String>,
        select: &[String],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), RpcError> {
        let slot_str = where_clauses
            .get("slot")
            .ok_or_else(|| RpcError::Transport("blocks requires slot".into()))?;
        let slot: u64 = slot_str
            .parse()
            .map_err(|_| RpcError::Transport("invalid slot".into()))?;
        let block = self.client.get_block(slot, "confirmed").await?;
        let row = flatten_block(slot, &block);
        let columns = pick_columns(select, &row);
        let values = columns.iter().map(|c| row.get(c).cloned().unwrap_or(Value::Null)).collect();
        Ok((columns, vec![values]))
    }

    async fn query_token_accounts(
        &self,
        where_clauses: &HashMap<String, String>,
        select: &[String],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), RpcError> {
        let owner = where_clauses.get("owner").map(|s| s.as_str());
        let mint = where_clauses.get("mint").map(|s| s.as_str());
        let owner = owner.ok_or_else(|| RpcError::Transport("token_accounts requires owner or mint".into()))?;
        let result = self
            .client
            .get_token_accounts_by_owner(owner, mint, "confirmed")
            .await?;
        let items = result["value"].as_array().cloned().unwrap_or_default();
        let rows: Vec<HashMap<String, Value>> = items.iter().map(|v| flatten_token_account(v)).collect();
        if rows.is_empty() {
            return Ok((vec!["pubkey".into()], vec![]));
        }
        let columns = pick_columns(select, &rows[0]);
        let values: Vec<Vec<Value>> = rows
            .iter()
            .map(|r| columns.iter().map(|c| r.get(c).cloned().unwrap_or(Value::Null)).collect())
            .collect();
        Ok((columns, values))
    }

    async fn query_program_accounts(
        &self,
        where_clauses: &HashMap<String, String>,
        select: &[String],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), RpcError> {
        let program_id = where_clauses
            .get("program_id")
            .ok_or_else(|| RpcError::Transport("program_accounts requires program_id".into()))?;
        let result = self
            .client
            .get_program_accounts(program_id, None, "confirmed")
            .await?;
        let items = result.as_array().cloned().unwrap_or_default();
        let mut rows = vec![];
        for item in items {
            let mut row = HashMap::new();
            row.insert("pubkey".into(), item["pubkey"].clone());
            row.insert("account".into(), item["account"].clone());
            rows.push(row);
        }
        if rows.is_empty() {
            return Ok((vec!["pubkey".into()], vec![]));
        }
        let columns = pick_columns(select, &rows[0]);
        let values: Vec<Vec<Value>> = rows
            .iter()
            .map(|r| columns.iter().map(|c| r.get(c).cloned().unwrap_or(Value::Null)).collect())
            .collect();
        Ok((columns, values))
    }

    async fn query_status(
        &self,
        select: &[String],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), RpcError> {
        let slot = self.client.get_slot("confirmed").await?;
        let height = self.client.get_block_height("confirmed").await?;
        let time = self.client.get_block_time(slot).await?;
        let mut row = HashMap::new();
        row.insert("slot".into(), json!(slot));
        row.insert("blockHeight".into(), json!(height));
        row.insert("blockTime".into(), json!(time));
        let columns = pick_columns(select, &row);
        let values = columns.iter().map(|c| row.get(c).cloned().unwrap_or(Value::Null)).collect();
        Ok((columns, vec![values]))
    }

    async fn query_epoch_info(
        &self,
        select: &[String],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), RpcError> {
        let info = self.client.get_epoch_info().await?;
        let row = flatten_dict(&info, "");
        let columns = pick_columns(select, &row);
        let values = columns.iter().map(|c| row.get(c).cloned().unwrap_or(Value::Null)).collect();
        Ok((columns, vec![values]))
    }

    async fn query_supply(
        &self,
        select: &[String],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), RpcError> {
        let supply = self.client.get_supply("confirmed", false).await?;
        let row = flatten_dict(&supply, "");
        let columns = pick_columns(select, &row);
        let values = columns.iter().map(|c| row.get(c).cloned().unwrap_or(Value::Null)).collect();
        Ok((columns, vec![values]))
    }

    async fn query_vote_accounts(
        &self,
        select: &[String],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), RpcError> {
        let result = self.client.get_vote_accounts().await?;
        let current = result["current"].as_array().cloned().unwrap_or_default();
        let delinquent = result["delinquent"].as_array().cloned().unwrap_or_default();
        let mut rows = vec![];
        for va in &current {
            let mut row = flatten_dict(va, "");
            row.insert("status".into(), json!("current"));
            rows.push(row);
        }
        for va in &delinquent {
            let mut row = flatten_dict(va, "");
            row.insert("status".into(), json!("delinquent"));
            rows.push(row);
        }
        if rows.is_empty() {
            return Ok((vec!["votePubkey".into()], vec![]));
        }
        let columns = pick_columns(select, &rows[0]);
        let values: Vec<Vec<Value>> = rows
            .iter()
            .map(|r| columns.iter().map(|c| r.get(c).cloned().unwrap_or(Value::Null)).collect())
            .collect();
        Ok((columns, values))
    }

    async fn query_cluster_nodes(
        &self,
        select: &[String],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), RpcError> {
        let nodes = self.client.get_cluster_nodes().await?;
        let items = nodes.as_array().cloned().unwrap_or_default();
        let rows: Vec<HashMap<String, Value>> = items.iter().map(|n| flatten_dict(n, "")).collect();
        if rows.is_empty() {
            return Ok((vec!["pubkey".into()], vec![]));
        }
        let columns = pick_columns(select, &rows[0]);
        let values: Vec<Vec<Value>> = rows
            .iter()
            .map(|r| columns.iter().map(|c| r.get(c).cloned().unwrap_or(Value::Null)).collect())
            .collect();
        Ok((columns, values))
    }

    async fn query_performance_samples(
        &self,
        limit: usize,
        select: &[String],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), RpcError> {
        let samples = self
            .client
            .get_recent_performance_samples(limit as u64)
            .await?;
        let items = samples.as_array().cloned().unwrap_or_default();
        let rows: Vec<HashMap<String, Value>> = items.iter().map(|s| flatten_dict(s, "")).collect();
        if rows.is_empty() {
            return Ok((vec!["slot".into()], vec![]));
        }
        let columns = pick_columns(select, &rows[0]);
        let values: Vec<Vec<Value>> = rows
            .iter()
            .map(|r| columns.iter().map(|c| r.get(c).cloned().unwrap_or(Value::Null)).collect())
            .collect();
        Ok((columns, values))
    }

    async fn query_token_supply(
        &self,
        where_clauses: &HashMap<String, String>,
        select: &[String],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), RpcError> {
        let mint = where_clauses
            .get("mint")
            .ok_or_else(|| RpcError::Transport("token_supply requires mint".into()))?;
        let supply = self.client.get_token_supply(mint, "confirmed").await?;
        let mut row = flatten_dict(&supply, "");
        row.insert("mint".into(), json!(mint));
        let columns = pick_columns(select, &row);
        let values = columns.iter().map(|c| row.get(c).cloned().unwrap_or(Value::Null)).collect();
        Ok((columns, vec![values]))
    }

    async fn query_inflation_reward(
        &self,
        where_clauses: &HashMap<String, String>,
        select: &[String],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), RpcError> {
        let pubkey = where_clauses
            .get("pubkey")
            .ok_or_else(|| RpcError::Transport("inflation_reward requires pubkey".into()))?;
        let epoch = where_clauses.get("epoch").and_then(|s| s.parse().ok());
        let rewards = self.client.get_inflation_reward(&[pubkey.clone()], epoch).await?;
        let arr = rewards.as_array().cloned().unwrap_or_default();
        let mut row = if let Some(first) = arr.first() {
            flatten_dict(first, "")
        } else {
            HashMap::new()
        };
        row.insert("pubkey".into(), json!(pubkey));
        let columns = pick_columns(select, &row);
        let values = columns.iter().map(|c| row.get(c).cloned().unwrap_or(Value::Null)).collect();
        Ok((columns, vec![values]))
    }

    async fn query_health(
        &self,
        select: &[String],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), RpcError> {
        let health = self.client.get_health().await?;
        let mut row = HashMap::new();
        row.insert("health".into(), json!(health));
        let columns = pick_columns(select, &row);
        let values = columns.iter().map(|c| row.get(c).cloned().unwrap_or(Value::Null)).collect();
        Ok((columns, vec![values]))
    }

    async fn query_version(
        &self,
        select: &[String],
    ) -> Result<(Vec<String>, Vec<Vec<Value>>), RpcError> {
        let version = self.client.get_version().await?;
        let row = flatten_dict(&version, "");
        let columns = pick_columns(select, &row);
        let values = columns.iter().map(|c| row.get(c).cloned().unwrap_or(Value::Null)).collect();
        Ok((columns, vec![values]))
    }
}

// ---- Helpers ----

fn pick_columns(select: &[String], row: &HashMap<String, Value>) -> Vec<String> {
    if select.contains(&"*".to_string()) {
        let mut keys: Vec<String> = row.keys().cloned().collect();
        keys.sort();
        keys
    } else {
        select.iter().cloned().collect()
    }
}

fn flatten_account(pubkey: &str, value: &Value) -> HashMap<String, Value> {
    let mut row = HashMap::new();
    row.insert("pubkey".into(), json!(pubkey));
    row.insert("lamports".into(), value["lamports"].clone());
    row.insert("owner".into(), value["owner"].clone());
    row.insert("executable".into(), value["executable"].clone());
    row.insert("rentEpoch".into(), value["rentEpoch"].clone());
    row.insert("space".into(), value["space"].clone());
    if let Some(parsed) = value["data"]["parsed"].as_object() {
        row.insert("parsed_type".into(), json!(parsed.get("type")));
        if let Some(info) = parsed["info"].as_object() {
            for (k, v) in info {
                row.insert(format!("info_{}", k), v.clone());
            }
        }
    }
    row
}

fn flatten_transaction(signature: &str, tx: &Value) -> HashMap<String, Value> {
    let mut row = HashMap::new();
    row.insert("signature".into(), json!(signature));
    row.insert("slot".into(), tx["slot"].clone());
    row.insert("blockTime".into(), tx["blockTime"].clone());
    row.insert("confirmationStatus".into(), tx["confirmationStatus"].clone());
    let meta = &tx["meta"];
    row.insert("err".into(), json!(meta["err"].as_object().map(|_| "true")));
    row.insert("fee".into(), meta["fee"].clone());
    row.insert("computeUnitsConsumed".into(), meta["computeUnitsConsumed"].clone());
    let message = &tx["transaction"]["message"];
    let num_inst = message["instructions"].as_array().map(|a| a.len()).unwrap_or(0);
    let num_signers = message["accountKeys"].as_array().map(|a| a.len()).unwrap_or(0);
    row.insert("numInstructions".into(), json!(num_inst));
    row.insert("numSigners".into(), json!(num_signers));
    row
}

fn flatten_block(slot: u64, block: &Value) -> HashMap<String, Value> {
    let mut row = HashMap::new();
    row.insert("slot".into(), json!(slot));
    row.insert("blockhash".into(), block["blockhash"].clone());
    row.insert("parentSlot".into(), block["parentSlot"].clone());
    row.insert("blockTime".into(), block["blockTime"].clone());
    row.insert("blockHeight".into(), block["blockHeight"].clone());
    let num_tx = block["transactions"].as_array().map(|a| a.len()).unwrap_or(0);
    let num_rewards = block["rewards"].as_array().map(|a| a.len()).unwrap_or(0);
    row.insert("numTransactions".into(), json!(num_tx));
    row.insert("rewards_count".into(), json!(num_rewards));
    row
}

fn flatten_token_account(acc: &Value) -> HashMap<String, Value> {
    let mut row = HashMap::new();
    row.insert("pubkey".into(), acc["pubkey"].clone());
    let account = &acc["account"];
    let parsed = &account["data"]["parsed"];
    if let Some(info) = parsed["info"].as_object() {
        row.insert("mint".into(), info.get("mint").cloned().unwrap_or(Value::Null));
        row.insert("owner".into(), info.get("owner").cloned().unwrap_or(Value::Null));
        let ta = info.get("tokenAmount").cloned().unwrap_or(Value::Null);
        row.insert(
            "tokenAmount".into(),
            ta["uiAmount"].clone(),
        );
        row.insert("decimals".into(), ta["decimals"].clone());
        row.insert("state".into(), info.get("state").cloned().unwrap_or(Value::Null));
    }
    row
}

fn flatten_dict(value: &Value, prefix: &str) -> HashMap<String, Value> {
    let mut result = HashMap::new();
    if let Some(obj) = value.as_object() {
        for (k, v) in obj {
            let key = if prefix.is_empty() {
                k.clone()
            } else {
                format!("{}{}", prefix, k)
            };
            if v.is_object() {
                for (kk, vv) in flatten_dict(v, &format!("{key}_")) {
                    result.insert(kk, vv);
                }
            } else if v.is_array() {
                result.insert(key, json!(v.as_array().unwrap().len()));
            } else {
                result.insert(key, v.clone());
            }
        }
    }
    result
}
