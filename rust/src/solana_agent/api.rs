use axum::{
    extract::{Query as AxumQuery, State, ws::WebSocketUpgrade},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, instrument};

use super::knowledge_base::SolanaKnowledgeBase;
use super::query_engine::QueryEngine;
use super::rpc_client::{RpcError, SolanaRpcClient};
use super::stream::SolanaStreamClient;

pub struct AppState {
    pub client: SolanaRpcClient,
    pub engine: QueryEngine,
    pub kb: SolanaKnowledgeBase,
}

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/version", get(version))
        .route("/query", post(sql_query_post))
        .route("/query", get(sql_query_get))
        .route("/rpc", post(rpc_call))
        .route("/account", post(account_info))
        .route("/balance", post(balance))
        .route("/transaction", post(transaction_info))
        .route("/block", post(block_info))
        .route("/slot", get(current_slot))
        .route("/epoch", get(epoch_info))
        .route("/supply", get(supply))
        .route("/token-accounts", post(token_accounts))
        .route("/program-accounts", post(program_accounts))
        .route("/cluster-nodes", get(cluster_nodes))
        .route("/vote-accounts", get(vote_accounts))
        .route("/performance", get(performance))
        .route("/ask", post(ask_question))
        .route("/topics", get(list_topics))
        .route("/ws/slots", get(ws_slots))
        .with_state(state)
}

// ---- Responses ----

fn ok<T: Serialize>(data: T) -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!(data)))
}

fn err(e: RpcError) -> (StatusCode, Json<Value>) {
    (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()})))
}

fn bad_request(msg: &str) -> (StatusCode, Json<Value>) {
    (StatusCode::BAD_REQUEST, Json(json!({"error": msg})))
}

// ---- Handlers ----

async fn root() -> Json<Value> {
    Json(json!({"service": "Solana Agent", "version": "0.1.0"}))
}

async fn health(State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>) {
    match state.client.get_health().await {
        Ok(h) => ok(json!({"status": "ok", "solana_health": h})),
        Err(e) => ok(json!({"status": "degraded", "solana_health": e.to_string()})),
    }
}

async fn version(State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>) {
    match state.client.get_version().await {
        Ok(v) => ok(v),
        Err(e) => err(e),
    }
}

#[derive(Debug, Deserialize)]
struct SqlQueryBody {
    query: String,
    #[serde(default)]
    params: Option<HashMap<String, String>>,
}

#[instrument(skip(state, body), fields(query = %body.query))]
async fn sql_query_post(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SqlQueryBody>,
) -> (StatusCode, Json<Value>) {
    debug!("Executing SQL query: {}", body.query);
    match state.engine.execute(&body.query, body.params).await {
        Ok(r) => {
            info!("Query executed successfully, {} rows", r.row_count);
            ok(r)
        }
        Err(RpcError::Transport(msg)) if msg.contains("requires") || msg.contains("Unknown") => {
            error!("Query parse error: {}", msg);
            bad_request(&msg)
        }
        Err(e) => {
            error!("Query execution error: {}", e);
            err(e)
        }
    }
}

#[derive(Debug, Deserialize)]
struct SqlQueryParams {
    q: String,
}

async fn sql_query_get(
    State(state): State<Arc<AppState>>,
    AxumQuery(params): AxumQuery<SqlQueryParams>,
) -> (StatusCode, Json<Value>) {
    match state.engine.execute(&params.q, None).await {
        Ok(r) => ok(r),
        Err(RpcError::Transport(msg)) if msg.contains("requires") || msg.contains("Unknown") => {
            bad_request(&msg)
        }
        Err(e) => err(e),
    }
}

#[derive(Debug, Deserialize)]
struct RpcBody {
    method: String,
    #[serde(default)]
    params: Vec<Value>,
    #[serde(default)]
    id: Value,
}

async fn rpc_call(
    State(state): State<Arc<AppState>>,
    Json(body): Json<RpcBody>,
) -> (StatusCode, Json<Value>) {
    match state.client.call(&body.method, json!(body.params)).await {
        Ok(result) => ok(json!({"jsonrpc": "2.0", "result": result, "id": body.id})),
        Err(e) => ok(json!({"jsonrpc": "2.0", "error": {"code": -32000, "message": e.to_string()}, "id": body.id})),
    }
}

#[derive(Debug, Deserialize)]
struct AccountReq {
    pubkey: String,
    #[serde(default = "default_commitment")]
    commitment: String,
}

async fn account_info(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AccountReq>,
) -> (StatusCode, Json<Value>) {
    match state.client.get_account_info(&req.pubkey, &req.commitment).await {
        Ok(v) => ok(v),
        Err(e) => err(e),
    }
}

#[derive(Debug, Deserialize)]
struct BalanceReq {
    pubkey: String,
    #[serde(default = "default_commitment")]
    commitment: String,
}

async fn balance(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BalanceReq>,
) -> (StatusCode, Json<Value>) {
    match state.client.get_balance(&req.pubkey, &req.commitment).await {
        Ok(lamports) => ok(json!({
            "pubkey": req.pubkey,
            "lamports": lamports,
            "sol": lamports as f64 / 1e9
        })),
        Err(e) => err(e),
    }
}

#[derive(Debug, Deserialize)]
struct TxReq {
    signature: String,
    #[serde(default = "default_commitment")]
    commitment: String,
}

async fn transaction_info(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TxReq>,
) -> (StatusCode, Json<Value>) {
    match state.client.get_transaction(&req.signature, &req.commitment).await {
        Ok(v) => ok(v),
        Err(e) => err(e),
    }
}

#[derive(Debug, Deserialize)]
struct BlockReq {
    slot: Option<u64>,
    #[serde(default = "default_commitment")]
    commitment: String,
}

async fn block_info(
    State(state): State<Arc<AppState>>,
    Json(req): Json<BlockReq>,
) -> (StatusCode, Json<Value>) {
    let slot = if let Some(s) = req.slot {
        s
    } else {
        match state.client.get_slot(&req.commitment).await {
            Ok(s) => s,
            Err(e) => return err(e),
        }
    };
    match state.client.get_block(slot, &req.commitment).await {
        Ok(v) => ok(v),
        Err(e) => err(e),
    }
}

#[derive(Debug, Deserialize)]
struct SlotQuery {
    #[serde(default = "default_commitment")]
    commitment: String,
}

fn default_commitment() -> String {
    "confirmed".into()
}

async fn current_slot(
    State(state): State<Arc<AppState>>,
    AxumQuery(q): AxumQuery<SlotQuery>,
) -> (StatusCode, Json<Value>) {
    match state.client.get_slot(&q.commitment).await {
        Ok(s) => ok(json!({"slot": s})),
        Err(e) => err(e),
    }
}

async fn epoch_info(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<Value>) {
    match state.client.get_epoch_info().await {
        Ok(v) => ok(v),
        Err(e) => err(e),
    }
}

#[derive(Debug, Deserialize)]
struct SupplyQuery {
    #[serde(default)]
    exclude_non_circulating: bool,
}

async fn supply(
    State(state): State<Arc<AppState>>,
    AxumQuery(q): AxumQuery<SupplyQuery>,
) -> (StatusCode, Json<Value>) {
    match state.client.get_supply("confirmed", q.exclude_non_circulating).await {
        Ok(v) => ok(v),
        Err(e) => err(e),
    }
}

#[derive(Debug, Deserialize)]
struct TokenAccountsReq {
    owner: Option<String>,
    mint: Option<String>,
}

async fn token_accounts(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TokenAccountsReq>,
) -> (StatusCode, Json<Value>) {
    let owner = req.owner.as_deref().unwrap_or("");
    let mint = req.mint.as_deref();
    match state
        .client
        .get_token_accounts_by_owner(owner, mint, "confirmed")
        .await
    {
        Ok(v) => ok(v),
        Err(e) => err(e),
    }
}

#[derive(Debug, Deserialize)]
struct ProgramAccountsReq {
    program_id: String,
    #[serde(default)]
    filters: Option<Value>,
}

async fn program_accounts(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ProgramAccountsReq>,
) -> (StatusCode, Json<Value>) {
    match state
        .client
        .get_program_accounts(&req.program_id, req.filters, "confirmed")
        .await
    {
        Ok(v) => ok(v),
        Err(e) => err(e),
    }
}

async fn cluster_nodes(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<Value>) {
    match state.client.get_cluster_nodes().await {
        Ok(v) => ok(v),
        Err(e) => err(e),
    }
}

async fn vote_accounts(
    State(state): State<Arc<AppState>>,
) -> (StatusCode, Json<Value>) {
    match state.client.get_vote_accounts().await {
        Ok(v) => ok(v),
        Err(e) => err(e),
    }
}

#[derive(Debug, Deserialize)]
struct PerfQuery {
    #[serde(default = "default_perf_limit")]
    limit: u64,
}

fn default_perf_limit() -> u64 {
    10
}

async fn performance(
    State(state): State<Arc<AppState>>,
    AxumQuery(q): AxumQuery<PerfQuery>,
) -> (StatusCode, Json<Value>) {
    match state.client.get_recent_performance_samples(q.limit).await {
        Ok(v) => ok(v),
        Err(e) => err(e),
    }
}

#[derive(Debug, Deserialize)]
struct AskReq {
    question: String,
    #[serde(default)]
    include_onchain_data: bool,
}

#[derive(Debug, Serialize)]
struct AskResp {
    answer: String,
    sources: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    onchain_data: Option<Value>,
}

async fn ask_question(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AskReq>,
) -> (StatusCode, Json<Value>) {
    let answer = state.kb.ask(&req.question);
    let mut resp = if let Some(a) = answer {
        AskResp {
            answer: a.answer,
            sources: a.sources,
            onchain_data: None,
        }
    } else {
        AskResp {
            answer: "I don't have a specific answer for that question. Try asking about: architecture, accounts, transactions, programs, tokens, staking, consensus, fees, PDAs, state compression, or security.".into(),
            sources: vec![],
            onchain_data: None,
        }
    };

    if req.include_onchain_data {
        if let Ok((slot, height, epoch)) = tokio::try_join!(
            state.client.get_slot("confirmed"),
            state.client.get_block_height("confirmed"),
            state.client.get_epoch_info()
        ) {
                let slot_index = epoch["slotIndex"].as_u64().unwrap_or(0);
                let slots_in_epoch = epoch["slotsInEpoch"].as_u64().unwrap_or(1).max(1);
                resp.onchain_data = Some(json!({
                    "slot": slot,
                    "blockHeight": height,
                    "epoch": epoch["epoch"],
                    "epochProgress": slot_index as f64 / slots_in_epoch as f64,
                }));
        }
    }

    ok(resp)
}

async fn list_topics(
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    Json(json!({"topics": state.kb.list_topics() }))
}

/// WebSocket handler for streaming slot updates
async fn ws_slots(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    ws.on_upgrade(|mut socket| async move {
        // Connect to Solana WebSocket
        let mut stream_client = SolanaStreamClient::new(state.client.endpoint().to_string());
        if let Err(e) = stream_client.connect().await {
            let _ = socket.send(axum::extract::ws::Message::Text(
                json!({"error": format!("Failed to connect to Solana WS: {}", e)}).to_string(),
            )).await;
            return;
        }

        // Subscribe to slots
        let mut slot_rx = match stream_client.subscribe_slots().await {
            Ok(rx) => rx,
            Err(e) => {
                let _ = socket.send(axum::extract::ws::Message::Text(
                    json!({"error": format!("Failed to subscribe: {}", e)}).to_string(),
                )).await;
                return;
            }
        };

        // Send initial confirmation
        let _ = socket.send(axum::extract::ws::Message::Text(
            json!({"status": "connected", "subscription": "slots"}).to_string(),
        )).await;

        // Forward slot events to WebSocket client
        while let Ok(event) = slot_rx.recv().await {
            let msg = json!({
                "type": "slot",
                "slot": event.slot,
                "timestamp": event.timestamp,
                "payload": event.payload,
            });
            if socket.send(axum::extract::ws::Message::Text(msg.to_string())).await.is_err() {
                break;
            }
        }
    })
}
