use llm_mina_chain::semantic_runtime::{Orchestrator, RuntimeContext};
use std::env;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt};

const BANNER: &str = r#"
   _____                      _       _   _               ____  _   _ _____ 
  / ____|                    | |     | | (_)             |  _ \| | | |_   _|
 | (___   ___  _ __ ___  __ _| |_ ___| |_ _ _ __   __ _  | |_) | | | | | |  
  \___ \ / _ \| '_ ` _ \/ _` | __/ _ \ __| | '_ \ / _` | |  _ <| | | | | |  
  ____) | (_) | | | | | | (_| | ||  __/ |_| | | | | (_| | | |_) | |_| |_| |_ 
 |_____/ \___/|_| |_| |_|\__,_|\__\___|\__|_|_| |_|\__, | |____/ \___/|_____|
                                                    __/ |                   
                                                   |___/                    
  Semantic Runtime v0.1.0 — AI-Native Blockchain OS
  Type 'help' for commands, 'exit' to quit.
"#;

const HELP: &str = r#"
Commands:
  <natural language>   Ask anything about Solana, your repo, or your system
  query <sql>          Execute SQL-like Solana query
  !<command>            Execute local shell command
  context              Show current runtime context
  memory               Show recent session actions
  receipt              Generate a Merkle root from session receipts
  help                 Show this help
  exit / quit          Exit

Examples:
  what is proof of history
  get account info for So11111111111111111111111111111111111111112
  SELECT * FROM status
  !cargo test
  analyze wallet <pubkey>
  what changed in my repo
"#;

#[tokio::main]
async fn main() {
    let endpoint = env::var("SOLANA_RPC_ENDPOINT").ok();
    let orchestrator = Orchestrator::new(endpoint.clone());

    // Gather context asynchronously
    let _context = RuntimeContext::gather().await; // gathered, Orchestrator holds its own copy

    let mut stdout = io::stdout();
    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin);
    let mut line = String::new();

    stdout.write_all(BANNER.as_bytes()).await.ok();
    stdout.flush().await.ok();

    loop {
        stdout.write_all(b"overllm> ").await.ok();
        stdout.flush().await.ok();
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match trimmed.to_lowercase().as_str() {
            "exit" | "quit" | "q" => {
                stdout.write_all(b"Goodbye.\n").await.ok();
                stdout.flush().await.ok();
                break;
            }
            "help" => {
                stdout.write_all(HELP.as_bytes()).await.ok();
                stdout.write_all(b"\n").await.ok();
                stdout.flush().await.ok();
            }
            "context" => {
                let ctx = RuntimeContext::gather().await;
                let json = serde_json::to_string_pretty(&ctx).unwrap_or_default();
                stdout.write_all(json.as_bytes()).await.ok();
                stdout.write_all(b"\n").await.ok();
                stdout.flush().await.ok();
            }
            "memory" => {
                let actions = orchestrator.memory.get_last_n(10);
                let json = serde_json::to_string_pretty(&actions).unwrap_or_default();
                stdout.write_all(json.as_bytes()).await.ok();
                stdout.write_all(b"\n").await.ok();
                stdout.flush().await.ok();
            }
            "receipt" => {
                match orchestrator.memory.merkle_root() {
                    Some(root) => {
                        let hex = hex::encode(root);
                        stdout
                            .write_all(format!("Session Merkle root: {}\n", hex).as_bytes())
                            .await
                            .ok();
                    }
                    None => {
                        stdout
                            .write_all(b"No receipts in session memory yet.\n")
                            .await
                            .ok();
                    }
                }
                stdout.flush().await.ok();
            }
            _ => {
                let result = orchestrator.execute(trimmed).await;
                stdout.write_all(result.as_bytes()).await.ok();
                stdout.write_all(b"\n").await.ok();
                stdout.flush().await.ok();
            }
        }
    }
}
