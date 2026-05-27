use serde_json::json;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt};
use tokio::signal;

use super::knowledge_base::SolanaKnowledgeBase;
use super::query_engine::QueryEngine;
use super::rpc_client::SolanaRpcClient;

const BANNER: &str = r#"
   _____       _                _         _           _         
  / ____|     | |              | |       | |         | |        
 | (___   ___ | | __ _ _ __ ___| | __    | | __ _  __| | ___    
  \___ \ / _ \| |/ _` | '__/ _ \ |/ /    | |/ _` |/ _` |/ _ \   
  ____) | (_) | | (_| | | |  __/   <     | | (_| | (_| |  __/   
 |_____/ \___/|_|\__,_|_|  \___|_|\_\    |_|\__,_|\__,_|\___|   
                                                                  
  Solana Agent CLI v0.1.0 — Type 'help' for commands
"#;

const HELP: &str = r#"
Commands:
  query <sql>          Execute a SQL-like query
  rpc <method> [args]  Call a raw Solana JSON-RPC method
  account <pubkey>     Get account info
  balance <pubkey>     Get SOL balance
  tx <signature>       Get transaction info
  block [slot]         Get block info (latest if no slot)
  slot                 Get current slot
  epoch                Get epoch info
  supply               Get total supply
  ask <question>       Ask the Solana knowledge base
  topics               List available knowledge topics
  health               Check Solana RPC health
  version              Get Solana version
  exit / quit          Exit

SQL Examples:
  query SELECT * FROM accounts WHERE pubkey = 'So11111111111111111111111111111111111111112'
  query SELECT * FROM transactions WHERE signature = '...'
  query SELECT * FROM blocks WHERE slot = 250000000
  query SELECT * FROM token_accounts WHERE owner = '...'
  query SELECT * FROM program_accounts WHERE program_id = 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'
  query SELECT * FROM status
  query SELECT * FROM epoch_info
  query SELECT * FROM supply
  query SELECT * FROM vote_accounts
  query SELECT * FROM cluster_nodes
  query SELECT * FROM performance_samples LIMIT 10
  query SELECT * FROM token_supply WHERE mint = '...'
"#;

pub async fn run_cli(endpoint: Option<String>) {
    let client = SolanaRpcClient::new(endpoint);
    let engine = QueryEngine::new(client.clone());
    let kb = SolanaKnowledgeBase::new();

    let mut stdout = io::stdout();
    let stdin = io::stdin();
    let mut reader = io::BufReader::new(stdin);
    let mut line = String::new();

    // Set up Ctrl-C handler using a broadcast channel
    let (shutdown_tx, mut shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);
    tokio::spawn(async move {
        if signal::ctrl_c().await.is_ok() {
            let _ = shutdown_tx.send(());
        }
    });

    stdout.write_all(BANNER.as_bytes()).await.ok();
    stdout
        .write_all(format!("Connected to: {}\n\n", client.endpoint()).as_bytes())
        .await
        .ok();
    stdout.flush().await.ok();

    loop {
        stdout.write_all(b"solana-agent> ").await.ok();
        stdout.flush().await.ok();
        line.clear();

        tokio::select! {
            _ = shutdown_rx.recv() => {
                stdout.write_all(b"\nReceived Ctrl-C. Exiting...\n").await.ok();
                stdout.flush().await.ok();
                break;
            }
            result = reader.read_line(&mut line) => {
                match result {
                    Ok(0) => {
                        // EOF received
                        stdout.write_all(b"\nEOF received. Exiting...\n").await.ok();
                        stdout.flush().await.ok();
                        break;
                    }
                    Ok(_) => {}
                    Err(_) => {
                        stdout.write_all(b"\nRead error. Exiting...\n").await.ok();
                        stdout.flush().await.ok();
                        break;
                    }
                }
            }
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
        let cmd = parts[0].to_lowercase();
        let rest = parts.get(1).unwrap_or(&"").trim();

        match cmd.as_str() {
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
            "query" => {
                if rest.is_empty() {
                    println!("Usage: query <sql>");
                    continue;
                }
                match engine.execute(rest, None).await {
                    Ok(result) => {
                        print_table(&result.columns, &result.rows);
                        println!(
                            "\nRows: {} | Time: {:.2}ms | Type: {}",
                            result.row_count, result.execution_time_ms, result.query_type
                        );
                    }
                    Err(e) => println!("Query error: {}", e),
                }
            }
            "rpc" => {
                let rpc_parts: Vec<&str> = rest.splitn(2, ' ').collect();
                let method = rpc_parts[0];
                let params_json = rpc_parts.get(1).unwrap_or(&"[]");
                let params: serde_json::Value = match serde_json::from_str(params_json) {
                    Ok(v) => v,
                    Err(_) => json!([]),
                };
                match client.call(method, params).await {
                    Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
                    Err(e) => println!("RPC error: {}", e),
                }
            }
            "account" => {
                if rest.is_empty() {
                    println!("Usage: account <pubkey>");
                    continue;
                }
                match client.get_account_info(rest, "confirmed").await {
                    Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
                    Err(e) => println!("Error: {}", e),
                }
            }
            "balance" => {
                if rest.is_empty() {
                    println!("Usage: balance <pubkey>");
                    continue;
                }
                match client.get_balance(rest, "confirmed").await {
                    Ok(lamports) => {
                        println!("Pubkey:   {}", rest);
                        println!("Lamports: {}", lamports);
                        println!("SOL:      {:.9}", lamports as f64 / 1e9);
                    }
                    Err(e) => println!("Error: {}", e),
                }
            }
            "tx" => {
                if rest.is_empty() {
                    println!("Usage: tx <signature>");
                    continue;
                }
                match client.get_transaction(rest, "confirmed").await {
                    Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
                    Err(e) => println!("Error: {}", e),
                }
            }
            "block" => {
                let slot: u64 = if rest.is_empty() {
                    match client.get_slot("confirmed").await {
                        Ok(s) => s,
                        Err(e) => {
                            println!("Error: {}", e);
                            continue;
                        }
                    }
                } else {
                    match rest.parse() {
                        Ok(s) => s,
                        Err(_) => {
                            println!("Invalid slot number");
                            continue;
                        }
                    }
                };
                match client.get_block(slot, "confirmed").await {
                    Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
                    Err(e) => println!("Error: {}", e),
                }
            }
            "slot" => {
                match tokio::try_join!(
                    client.get_slot("confirmed"),
                    client.get_block_height("confirmed")
                ) {
                    Ok((slot, height)) => {
                        println!("Current slot:       {}", slot);
                        println!("Current block height: {}", height);
                    }
                    Err(e) => println!("Error: {}", e),
                }
            }
            "epoch" => {
                match client.get_epoch_info().await {
                    Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
                    Err(e) => println!("Error: {}", e),
                }
            }
            "supply" => {
                match client.get_supply("confirmed", false).await {
                    Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
                    Err(e) => println!("Error: {}", e),
                }
            }
            "ask" => {
                if rest.is_empty() {
                    println!("Usage: ask <question>");
                    continue;
                }
                if let Some(ans) = kb.ask(rest) {
                    println!("{}", ans.answer);
                    if !ans.sources.is_empty() {
                        println!("\nSources: {}", ans.sources.join(", "));
                    }
                } else {
                    println!("I don't have a specific answer for that.");
                    println!("Try asking about: architecture, accounts, transactions, programs, tokens, staking, consensus, fees, PDAs, state compression, or security.");
                }
            }
            "topics" => {
                println!("Available topics:");
                for t in kb.list_topics() {
                    println!("  - {}", t);
                }
            }
            "health" => {
                match client.get_health().await {
                    Ok(h) => println!("RPC Health: {}", h),
                    Err(e) => println!("RPC Health: UNHEALTHY ({})", e),
                }
            }
            "version" => {
                match client.get_version().await {
                    Ok(v) => println!("{}", serde_json::to_string_pretty(&v).unwrap()),
                    Err(e) => println!("Error: {}", e),
                }
            }
            _ => println!("Unknown command: {}. Type 'help' for available commands.", cmd),
        }
    }
}

fn print_table(columns: &[String], rows: &[Vec<serde_json::Value>]) {
    if rows.is_empty() {
        println!("(no results)");
        return;
    }
    let mut widths: Vec<usize> = columns.iter().map(|c| c.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            let s = format!("{}", cell);
            widths[i] = widths[i].max(s.chars().take(80).collect::<String>().len());
        }
    }
    let header: String = columns
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{:<width$}", c, width = widths[i]))
        .collect::<Vec<_>>()
        .join(" | ");
    println!("{}", header);
    println!("{}", "-".repeat(header.len()));
    for row in rows {
        let line: String = row
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                let s = format!("{}", cell);
                let truncated: String = s.chars().take(80).collect();
                format!("{:<width$}", truncated, width = widths[i])
            })
            .collect::<Vec<_>>()
            .join(" | ");
        println!("{}", line);
    }
}
