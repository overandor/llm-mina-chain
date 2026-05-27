use llm_mina_chain::solana_agent::run_cli;
use std::env;

#[tokio::main]
async fn main() {
    let endpoint = env::var("SOLANA_RPC_ENDPOINT").ok();
    run_cli(endpoint).await;
}
