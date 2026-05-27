use llm_mina_chain::solana_agent::{build_router, QueryEngine, SolanaKnowledgeBase, SolanaRpcClient};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;

fn bind_addr() -> SocketAddr {
    if let Ok(addr) = env::var("BIND_ADDR") {
        if let Ok(parsed) = addr.parse() {
            return parsed;
        }
    }

    // Render provides PORT and expects the service to listen on 0.0.0.0.
    if let Ok(port) = env::var("PORT") {
        if let Ok(port_num) = port.parse::<u16>() {
            return SocketAddr::from(([0, 0, 0, 0], port_num));
        }
    }

    "0.0.0.0:8000".parse().unwrap()
}

#[tokio::main]
async fn main() {
    let endpoint = env::var("SOLANA_RPC_ENDPOINT").ok();
    let client = SolanaRpcClient::new(endpoint);
    let engine = QueryEngine::new(client.clone());
    let kb = SolanaKnowledgeBase::new();

    let state = Arc::new(llm_mina_chain::solana_agent::api::AppState {
        client,
        engine,
        kb,
    });

    let app = build_router(state);
    let addr = bind_addr();

    println!("Solana Agent server listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
