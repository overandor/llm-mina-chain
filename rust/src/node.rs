//! Blockchain node for LLM-Mina-Chain

use llm_mina_chain::{Blockchain, Transaction, LLMTransactionParser};
use std::io::{self, BufRead};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    println!("🔗 LLM-Mina-Chain Node v0.1.0");
    println!("================================\n");
    
    // Initialize blockchain
    let blockchain = Arc::new(Mutex::new(Blockchain::new()));
    let parser = LLMTransactionParser::new();
    
    println!("✅ Blockchain initialized");
    println!("📊 Current state:");
    print_state(&blockchain.lock().unwrap());
    
    // Start mining thread
    let blockchain_clone = blockchain.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(5));
            let mut bc = blockchain_clone.lock().unwrap();
            
            if !bc.transaction_pool.is_empty() {
                let txs = bc.transaction_pool.clone();
                match bc.create_block(txs) {
                    Some(block) => {
                        println!("⛏️  Mined block #{} with {} transactions", block.height, block.transactions.len());
                    }
                    None => {
                        println!("❌ Failed to create block - invalid transactions");
                    }
                }
            }
        }
    });
    
    // Main loop
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let input = line.unwrap();
        let input = input.trim();
        
        if input.is_empty() {
            continue;
        }
        
        handle_command(input, &blockchain, &parser);
    }
}

fn handle_command(input: &str, blockchain: &Arc<Mutex<Blockchain>>, parser: &LLMTransactionParser) {
    let parts: Vec<&str> = input.split_whitespace().collect();
    
    match parts.get(0).map(|s| *s) {
        Some("help") => print_help(),
        Some("state") => {
            let bc = blockchain.lock().unwrap();
            print_state(&bc);
        }
        Some("block") => {
            if let Some(height_str) = parts.get(1) {
                if let Ok(height) = height_str.parse::<u64>() {
                    let bc = blockchain.lock().unwrap();
                    if let Some(block) = bc.get_block(height) {
                        print_block(block);
                    } else {
                        println!("❌ Block #{} not found", height);
                    }
                }
            } else {
                let bc = blockchain.lock().unwrap();
                print_block(bc.get_latest_block());
            }
        }
        Some("chain") => {
            let bc = blockchain.lock().unwrap();
            print_chain(&bc);
        }
        Some("transfer") => {
            if parts.len() >= 4 {
                let sender = parts[1];
                let receiver = parts[2];
                let amount: u64 = parts[3].parse().unwrap_or(0);
                
                let mut bc = blockchain.lock().unwrap();
                let nonce = bc.state.get_nonce(sender);
                
                let tx = Transaction::new(
                    sender.to_string(),
                    receiver.to_string(),
                    amount,
                    nonce,
                    Some(21000),
                    Some(1),
                );
                
                if bc.add_transaction(tx.clone()) {
                    println!("✅ Transaction added to pool: {}", tx.tx_id);
                    println!("   {} -> {} ({})", sender, receiver, amount);
                } else {
                    println!("❌ Transaction validation failed");
                }
            } else {
                println!("Usage: transfer <sender> <receiver> <amount>");
            }
        }
        Some("gasless") => {
            if parts.len() >= 4 {
                let sender = parts[1];
                let receiver = parts[2];
                let amount: u64 = parts[3].parse().unwrap_or(0);
                
                let mut bc = blockchain.lock().unwrap();
                let nonce = bc.state.get_nonce(sender);
                
                let tx = Transaction::new(
                    sender.to_string(),
                    receiver.to_string(),
                    amount,
                    nonce,
                    None,  // gasless
                    None,
                );
                
                if bc.add_transaction(tx.clone()) {
                    println!("✅ Gasless transaction added to pool: {}", tx.tx_id);
                    println!("   {} -> {} ({})", sender, receiver, amount);
                } else {
                    println!("❌ Transaction validation failed");
                }
            } else {
                println!("Usage: gasless <sender> <receiver> <amount>");
            }
        }
        Some("llm") => {
            if parts.len() >= 2 {
                let text = parts[1..].join(" ");
                let bc = blockchain.lock().unwrap();
                
                let parsed = parser.parse(&text, Some("alice"));
                println!("🤖 Parsed transaction:");
                println!("   Confidence: {:.1}", parsed.confidence);
                println!("   Explanation: {}", parsed.explanation);
                println!("   Sender: {:?}", parsed.sender);
                println!("   Receiver: {}", parsed.receiver);
                println!("   Amount: {}", parsed.amount);
                println!("   Gas: {:?}", parsed.gas_limit);
                
                if parsed.confidence > 0.5 {
                    let nonce = bc.state.get_nonce(parsed.sender.as_deref().unwrap_or("alice"));
                    let tx = Transaction::new(
                        parsed.sender.unwrap_or_else(|| "alice".to_string()),
                        parsed.receiver,
                        parsed.amount,
                        nonce,
                        parsed.gas_limit,
                        parsed.gas_price,
                    );
                    
                    drop(bc);
                    let mut bc = blockchain.lock().unwrap();
                    if bc.add_transaction(tx.clone()) {
                        println!("✅ Transaction added to pool: {}", tx.tx_id);
                    } else {
                        println!("❌ Transaction validation failed");
                    }
                }
            } else {
                println!("Usage: llm <natural language command>");
                println!("Example: llm transfer 100 from alice to bob");
                println!("Example: llm send 50 to bob gasless");
            }
        }
        Some("mine") => {
            let mut bc = blockchain.lock().unwrap();
            let txs = bc.transaction_pool.clone();
            
            if txs.is_empty() {
                println!("⚠️  No transactions in pool");
                return;
            }
            
            match bc.create_block(txs) {
                Some(block) => {
                    println!("⛏️  Mined block #{} with {} transactions", block.height, block.transactions.len());
                    print_block(&block);
                }
                None => {
                    println!("❌ Failed to create block - invalid transactions");
                }
            }
        }
        Some("pool") => {
            let bc = blockchain.lock().unwrap();
            println!("📦 Transaction Pool ({} transactions)", bc.transaction_pool.len());
            for tx in &bc.transaction_pool {
                println!("   {} -> {} ({}) [{}]", tx.sender, tx.receiver, tx.amount, &tx.tx_id[..8]);
            }
        }
        Some("gas") => {
            if let Some(price_str) = parts.get(1) {
                if let Ok(price) = price_str.parse::<u64>() {
                    let mut bc = blockchain.lock().unwrap();
                    bc.set_gas_price(price);
                    println!("⛽ Gas price set to {}", price);
                }
            } else {
                let bc = blockchain.lock().unwrap();
                println!("⛽ Current gas price: {}", bc.get_gas_price());
            }
        }
        Some("exit") | Some("quit") => {
            println!("👋 Goodbye!");
            std::process::exit(0);
        }
        _ => {
            println!("❓ Unknown command. Type 'help' for available commands.");
        }
    }
}

fn print_help() {
    println!("📖 Available Commands:");
    println!("   help              - Show this help");
    println!("   state             - Show current blockchain state");
    println!("   block [height]    - Show specific block or latest");
    println!("   chain             - Show entire blockchain");
    println!("   transfer <s> <r> <a>  - Create transfer transaction");
    println!("   gasless <s> <r> <a>   - Create gasless transaction");
    println!("   llm <text>        - Parse natural language to transaction");
    println!("   mine              - Mine next block");
    println!("   pool              - Show transaction pool");
    println!("   gas [price]       - Set or get gas price");
    println!("   exit              - Exit the node");
}

fn print_state(bc: &Blockchain) {
    println!("   Balances:");
    for (addr, balance) in &bc.state.balances {
        println!("     {}: {}", addr, balance);
    }
    println!("   Nonces:");
    for (addr, nonce) in &bc.state.nonces {
        println!("     {}: {}", addr, nonce);
    }
    println!("   Gas Price: {}", bc.gas_price);
}

fn print_block(block: &llm_mina_chain::Block) {
    println!("📦 Block #{}", block.height);
    println!("   Hash: {}", &block.block_hash[..64.min(block.block_hash.len())]);
    println!("   Previous: {}", &block.previous_hash[..64.min(block.previous_hash.len())]);
    println!("   State Hash: {}", &block.state_hash[..64.min(block.state_hash.len())]);
    println!("   Proof: {:?}", block.proof);
    println!("   Transactions: {}", block.transactions.len());
    for tx in &block.transactions {
        println!("     {} -> {} ({}) [{}]", tx.sender, tx.receiver, tx.amount, &tx.tx_id[..8]);
    }
}

fn print_chain(bc: &Blockchain) {
    println!("🔗 Blockchain ({} blocks)", bc.chain.len());
    for block in &bc.chain {
        println!("   Block #{}: {} transactions, hash: {}", 
            block.height, 
            block.transactions.len(),
            &block.block_hash[..16.min(block.block_hash.len())]
        );
    }
}
