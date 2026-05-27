//! LLM integration layer for natural language transaction generation and validation

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use crate::{Transaction, State};

/// Parsed transaction from natural language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTransaction {
    pub sender: Option<String>,
    pub receiver: String,
    pub amount: u64,
    pub gas_limit: Option<u64>,
    pub gas_price: Option<u64>,
    pub tx_type: String,
    pub data: Option<serde_json::Value>,
    pub confidence: f64,
    pub explanation: String,
}

/// LLM transaction parser
pub struct LLMTransactionParser {
    accounts: HashSet<String>,
    transfer_patterns: Vec<Regex>,
    gas_patterns: Vec<Regex>,
}

impl LLMTransactionParser {
    pub fn new() -> Self {
        let accounts: HashSet<String> = [
            "alice".to_string(),
            "bob".to_string(),
            "charlie".to_string(),
            "genesis".to_string(),
        ].iter().cloned().collect();
        
        let transfer_patterns = vec![
            Regex::new(r"transfer\s+(\d+)\s+from\s+(\w+)\s+to\s+(\w+)").unwrap(),
            Regex::new(r"send\s+(\d+)\s+from\s+(\w+)\s+to\s+(\w+)").unwrap(),
            Regex::new(r"(\w+)\s+sends?\s+(\d+)\s+to\s+(\w+)").unwrap(),
            Regex::new(r"pay\s+(\w+)\s+(\d+)").unwrap(),
            Regex::new(r"give\s+(\w+)\s+(\d+)").unwrap(),
        ];
        
        let gas_patterns = vec![
            Regex::new(r"gas\s+limit\s+(\d+)").unwrap(),
            Regex::new(r"gas\s+price\s+(\d+)").unwrap(),
            Regex::new(r"with\s+(\d+)\s+gas").unwrap(),
        ];
        
        LLMTransactionParser {
            accounts,
            transfer_patterns,
            gas_patterns,
        }
    }
    
    /// Parse natural language into transaction
    pub fn parse(&self, text: &str, default_sender: Option<&str>) -> ParsedTransaction {
        let text = text.to_lowercase();
        
        let mut parsed = ParsedTransaction {
            sender: default_sender.map(|s| s.to_string()),
            receiver: String::new(),
            amount: 0,
            gas_limit: None,
            gas_price: None,
            tx_type: "transfer".to_string(),
            data: None,
            confidence: 0.0,
            explanation: String::new(),
        };
        
        // Try to match transfer patterns
        for pattern in &self.transfer_patterns {
            if let Some(captures) = pattern.captures(&text) {
                let groups: Vec<&str> = captures.iter().map(|c| c.map(|m| m.as_str()).unwrap_or("")).collect();
                
                if groups.len() >= 4 {
                    // Pattern: transfer X from A to B
                    parsed.amount = groups[1].parse().unwrap_or(0);
                    parsed.sender = if self.accounts.contains(groups[2]) {
                        Some(groups[2].to_string())
                    } else {
                        default_sender.map(|s| s.to_string())
                    };
                    parsed.receiver = groups[3].to_string();
                } else if groups.len() >= 3 {
                    // Pattern: A sends X to B or pay B X
                    if self.accounts.contains(groups[1]) {
                        parsed.sender = Some(groups[1].to_string());
                        parsed.amount = groups[2].parse().unwrap_or(0);
                    } else {
                        parsed.receiver = groups[1].to_string();
                        parsed.amount = groups[2].parse().unwrap_or(0);
                        parsed.sender = default_sender.map(|s| s.to_string());
                    }
                }
                
                parsed.confidence = 0.8;
                parsed.explanation = format!(
                    "Transfer {} from {} to {}",
                    parsed.amount,
                    parsed.sender.as_deref().unwrap_or("unknown"),
                    parsed.receiver
                );
                break;
            }
        }
        
        // Check for gasless
        if text.contains("gasless") || text.contains("no gas") {
            parsed.gas_limit = None;
            parsed.gas_price = None;
            parsed.explanation += " (gasless transaction)";
        } else {
            // Try to match gas patterns
            for pattern in &self.gas_patterns {
                if let Some(captures) = pattern.captures(&text) {
                    if let Some(limit) = captures.get(1) {
                        if text.contains("gas limit") {
                            parsed.gas_limit = Some(limit.as_str().parse().unwrap_or(0));
                        } else if text.contains("gas price") {
                            parsed.gas_price = Some(limit.as_str().parse().unwrap_or(0));
                        }
                    }
                }
            }
        }
        
        // Validate receiver
        if !parsed.receiver.is_empty() && !self.accounts.contains(&parsed.receiver) {
            parsed.confidence = (parsed.confidence - 0.3).max(0.0);
            parsed.explanation += &format!(" (warning: receiver '{}' not known)", parsed.receiver);
        }
        
        parsed
    }
    
    /// Validate transaction semantics
    /// Economic model: sender mines (does work), receiver optionally pays gas
    pub fn validate_semantics(&self, tx: &Transaction, state: &State) -> (bool, String) {
        let mut issues = Vec::new();
        
        // Check amount
        if tx.amount == 0 {
            issues.push("Amount must be positive".to_string());
        }
        
        if tx.amount > 1_000_000 {
            issues.push("Amount suspiciously large".to_string());
        }
        
        // Check gas parameters
        if let Some(limit) = tx.gas_limit {
            if limit < 21_000 {
                issues.push("Gas limit too low".to_string());
            }
        }
        
        if let Some(price) = tx.gas_price {
            if price > 1_000 {
                issues.push("Gas price suspiciously high".to_string());
            }
        }
        
        // Check sender/receiver are different
        if tx.sender == tx.receiver {
            issues.push("Sender and receiver cannot be the same".to_string());
        }
        
        // Check context (balance, nonce)
        // Sender must have enough to send the amount (mining work)
        let sender_balance = state.get_balance(&tx.sender);
        if sender_balance < tx.amount {
            issues.push(format!("Insufficient sender balance: {} < {}", sender_balance, tx.amount));
        }
        
        // Receiver must afford gas (if gas is specified)
        let gas_cost = tx.calculate_gas_cost();
        if gas_cost > 0 {
            let receiver_balance = state.get_balance(&tx.receiver);
            if receiver_balance < gas_cost {
                issues.push(format!("Insufficient receiver balance for gas: {} < {}", receiver_balance, gas_cost));
            }
        }
        
        let sender_nonce = state.get_nonce(&tx.sender);
        if tx.nonce != sender_nonce {
            issues.push(format!("Invalid nonce: expected {}, got {}", sender_nonce, tx.nonce));
        }
        
        if issues.is_empty() {
            (true, "Transaction semantics valid".to_string())
        } else {
            (false, issues.join("; "))
        }
    }
    
    /// Suggest gas parameters
    pub fn suggest_gas(&self, tx: &Transaction) -> GasSuggestion {
        let base_gas = match tx.tx_type.as_str() {
            "contract_call" => 100_000,
            _ => 21_000,
        };
        
        GasSuggestion {
            gas_limit: base_gas,
            gas_price: 1,
        }
    }
    
    /// Generate natural language explanation
    pub fn explain(&self, tx: &Transaction) -> String {
        let gas_info = if tx.is_gasless() {
            " (gasless)".to_string()
        } else {
            format!(" (gas: {} * {} = {})", 
                tx.gas_limit.unwrap_or(0), 
                tx.gas_price.unwrap_or(0), 
                tx.calculate_gas_cost())
        };
        
        let mut explanation = format!(
            "Transaction {}: {} sends {} to {}{}",
            &tx.tx_id[..8.min(tx.tx_id.len())],
            tx.sender,
            tx.amount,
            tx.receiver,
            gas_info
        );
        
        if let Some(data) = &tx.data {
            explanation += &format!(" with data: {}", serde_json::to_string(data).unwrap());
        }
        
        explanation
    }
}

impl Default for LLMTransactionParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Gas suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasSuggestion {
    pub gas_limit: u64,
    pub gas_price: u64,
}
